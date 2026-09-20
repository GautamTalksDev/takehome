#!/usr/bin/env node
/**
 * Link checker for markdown and Astro docs. Internal and external.
 * External failures are errors unless --offline (then external is warn-only skip).
 */
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const OFFLINE = process.argv.includes('--offline');
const SKIP_EXTERNAL = process.argv.includes('--skip-external') || OFFLINE;

const SKIP_DIRS = new Set([
  'node_modules',
  'target',
  'dist',
  '.git',
  'pdoc-cache',
  'pdoc-screenshots',
  'vendor',
]);

function walk(dir, pred, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, pred, acc);
    else if (pred(name)) acc.push(full);
  }
  return acc;
}

function extractLinks(source) {
  const links = [];
  const md = /\[([^\]]*)\]\(([^)]+)\)/g;
  let m;
  while ((m = md.exec(source)) !== null) {
    links.push(m[2].split(/\s+/)[0].replace(/^<|>$/g, ''));
  }
  const href = /href=["']([^"']+)["']/g;
  while ((m = href.exec(source)) !== null) {
    links.push(m[1]);
  }
  const src = /src=["']([^"']+)["']/g;
  while ((m = src.exec(source)) !== null) {
    links.push(m[1]);
  }
  return links;
}

function resolveInternal(fromFile, href) {
  if (href.startsWith('#')) return { ok: true, kind: 'hash' };
  if (href.startsWith('mailto:')) return { ok: true, kind: 'mailto' };
  const clean = href.split('#')[0].split('?')[0];
  if (!clean) return { ok: true, kind: 'hash' };
  // Placeholder templates in docs/diagrams/_include.md
  if (clean.includes('ID.svg') || clean.includes('/diagrams/ID')) {
    return { ok: true, kind: 'template' };
  }
  if (clean.startsWith('/')) {
    // Site path: check public/ or pages/
    const pub = path.join(ROOT, 'web/site/public', clean.replace(/^\//, ''));
    if (existsSync(pub)) return { ok: true, kind: 'public' };
    const trimmed = clean.replace(/\/$/, '').replace(/^\//, '');
    const asFile = path.join(ROOT, 'web/site/src/pages', `${trimmed}.astro`);
    const asIndex = path.join(ROOT, 'web/site/src/pages', trimmed, 'index.astro');
    if (existsSync(asFile) || existsSync(asIndex)) return { ok: true, kind: 'page' };
    const docsMd = path.join(ROOT, 'docs', `${trimmed.replace(/^docs\//, '')}.md`);
    if (existsSync(docsMd)) return { ok: true, kind: 'docs-md' };
    const known = new Set([
      'conformance',
      'docs',
      'changes',
      'changes.xml',
      'pricing',
      'signup',
      'embed',
      'calculators',
      'what-is-the-t4127',
      'cpp-2027-rate-change',
      'how-payroll-deductions-work-in-canada',
      'gross-up-calculator', // catalog situation page from templates
    ]);
    const head = trimmed.split('/')[0];
    if (
      known.has(trimmed) ||
      known.has(head) ||
      trimmed.startsWith('calculators/') ||
      trimmed.startsWith('embed/') ||
      trimmed.startsWith('v1/') ||
      trimmed.startsWith('rates/') ||
      trimmed.startsWith('signup/') ||
      trimmed.startsWith('docs/') ||
      trimmed.startsWith('diagrams/')
    ) {
      if (trimmed.startsWith('diagrams/')) {
        const svg = path.join(ROOT, 'web/site/public', trimmed);
        return { ok: existsSync(svg), kind: 'diagram', path: svg };
      }
      return { ok: true, kind: 'route' };
    }
    return { ok: false, kind: 'missing-route', path: clean };
  }
  // Relative from markdown file, or repo-root-style docs/... from nested tools paths
  const base = path.dirname(fromFile);
  const candidates = [
    path.resolve(base, clean),
    path.resolve(base, clean + '.md'),
    path.join(ROOT, clean),
    path.join(ROOT, clean + '.md'),
  ];
  for (const target of candidates) {
    if (existsSync(target)) return { ok: true, kind: 'file' };
  }
  return { ok: false, kind: 'missing-file', path: candidates[0] };
}

async function checkExternal(url) {
  try {
    const res = await fetch(url, {
      method: 'HEAD',
      redirect: 'follow',
      signal: AbortSignal.timeout(15000),
      headers: { 'user-agent': 'Takehome-Docs-LinkCheck/0.1' },
    });
    if (res.ok || res.status === 405 || res.status === 403) return { ok: true, status: res.status };
    // Some hosts reject HEAD; try GET
    const get = await fetch(url, {
      method: 'GET',
      redirect: 'follow',
      signal: AbortSignal.timeout(15000),
      headers: { 'user-agent': 'Takehome-Docs-LinkCheck/0.1' },
    });
    return { ok: get.ok || get.status === 403, status: get.status };
  } catch (err) {
    return { ok: false, error: String(err) };
  }
}

async function main() {
  const files = [
    ...walk(ROOT, (n) => n.endsWith('.md')),
    ...walk(path.join(ROOT, 'web/site/src/pages'), (n) => n.endsWith('.astro')),
  ].filter((f) => {
    const rel = path.relative(ROOT, f);
    if (rel.startsWith('crates/') || rel.startsWith('packages/') || rel.startsWith('tools/')) {
      return rel.startsWith('tools/conformance/src/methodology.md');
    }
    if (rel.startsWith('.cursor/')) return false;
    return true;
  });

  const errors = [];
  const external = new Map();

  for (const file of files) {
    const rel = path.relative(ROOT, file);
    const source = readFileSync(file, 'utf8');
    for (const href of extractLinks(source)) {
      if (/^(javascript:|data:)/.test(href)) continue;
      if (/^https?:\/\//i.test(href)) {
        if (!external.has(href)) external.set(href, []);
        external.get(href).push(rel);
        continue;
      }
      const result = resolveInternal(file, href);
      if (!result.ok) {
        errors.push(`${rel}: broken internal link ${href} (${result.kind})`);
      }
    }
  }

  if (!SKIP_EXTERNAL) {
    for (const [url, from] of external) {
      const result = await checkExternal(url);
      if (!result.ok) {
        errors.push(
          `external ${url} failed (${result.status ?? result.error}); referenced from ${from.slice(0, 3).join(', ')}`,
        );
      }
    }
  } else {
    console.log(`link-check: skipped ${external.size} external URL(s) (--skip-external)`);
  }

  if (errors.length) {
    for (const e of errors) console.error(`FAIL ${e}`);
    console.error(`link-check: ${errors.length} error(s)`);
    process.exit(1);
  }
  console.log(
    `link-check: clean (${files.length} files, ${external.size} unique external URL(s))`,
  );
}

main();
