#!/usr/bin/env node
/**
 * Documentation house style (items 11–17, 46, 49).
 * Failures exit 1. Sentence length over 35 words is a warning (exit 0 unless
 * --strict-sentences). Fenced and inline code are skipped for punctuation
 * and banned-word checks.
 */
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const STRICT_SENTENCES = process.argv.includes('--strict-sentences');
const WARN_WORDS = 35;
const TARGET_WORDS = 25;

const BANNED = /\b(simply|just|obviously|of course|easy|trivial)\b/gi;

const TERMS = [
  {
    id: 'T4127',
    re: /\bT4127\b/,
    defined: /T4127\s*\([^)]*(?:Payroll Deductions Formulas|payroll deduction formulas)[^)]*\)/i,
  },
  {
    id: 'YMPE',
    re: /\bYMPE\b/,
    defined: /YMPE\s*\([^)]*(?:Year'?s Maximum Pensionable Earnings|maximum pensionable)[^)]*\)/i,
  },
  {
    id: 'YAMPE',
    re: /\bYAMPE\b/,
    defined: /YAMPE\s*\([^)]*(?:Year'?s Additional Maximum Pensionable Earnings|additional maximum)[^)]*\)/i,
  },
  {
    id: 'BPAF',
    re: /\bBPAF\b/,
    defined: /BPAF\s*\([^)]*(?:Basic Personal Amount|basic personal amount)[^)]*\)/i,
  },
  {
    id: 'CPP2',
    re: /\bCPP2\b/,
    defined: /CPP2\s*\([^)]*(?:second additional|CPP additional|additional CPP)[^)]*\)/i,
  },
  {
    id: 'QPIP',
    re: /\bQPIP\b/,
    defined: /QPIP\s*\([^)]*(?:Québec Parental|Quebec Parental|parental insurance)[^)]*\)/i,
  },
  {
    id: 'proration',
    re: /\bproration\b/i,
    defined:
      /\bproration\b[^.!?\n]{0,120}\b(split|mid-year|half|average|OptionScoped|true annual)/i,
  },
  {
    id: 'claim code',
    re: /\bclaim codes?\b/i,
    defined:
      /\bclaim codes?\b[^.!?\n]{0,120}\b(TD1|personal amount|CRA claim|federal and provincial)/i,
  },
  {
    id: 'effective date',
    re: /\beffective dates?\b/i,
    defined:
      /\beffective dates?\b[^.!?\n]{0,140}\b(as_of|rule set|which rules|calendar date)/i,
  },
];

const SKIP_DIRS = new Set([
  'node_modules',
  'target',
  'dist',
  '.git',
  'vendor',
  'pdoc-cache',
  'pdoc-screenshots',
]);

const SKIP_FILES = new Set([
  // Generated; regenerated from tools/conformance. Lint after regenerate.
]);

/** Strip fenced ``` blocks and `inline` code for prose checks. */
export function stripCode(source) {
  let out = source.replace(/```[\s\S]*?```/g, '\n');
  out = out.replace(/`[^`\n]+`/g, ' CODE ');
  // Markdown table separator rows are not prose punctuation.
  out = out
    .split('\n')
    .filter((line) => !/^\s*\|?[\s:-]*-{2,}[\s|:-]*$/.test(line))
    .join('\n');
  return out;
}

/** Remove HTML comments, YAML frontmatter, script/style, and tags (Astro/HTML). */
export function stripNoise(source) {
  let out = source.replace(/^---\n[\s\S]*?\n---\n/, '\n');
  out = out.replace(/<!--[\s\S]*?-->/g, '\n');
  out = out.replace(/<script\b[\s\S]*?<\/script>/gi, '\n');
  out = out.replace(/<style\b[\s\S]*?<\/style>/gi, '\n');
  out = out.replace(/\{[\s\S]*?\}/g, ' '); // Astro expressions
  out = out.replace(/<[^>]+>/g, ' ');
  return out;
}

export function findEmDashes(prose) {
  const hits = [];
  const re = /[—–]/g;
  let m;
  while ((m = re.exec(prose)) !== null) {
    hits.push({ index: m.index, text: m[0] });
  }
  return hits;
}

/** Double hyphen used as punctuation (space--space or word--word), not flags. */
export function findPunctuationDoubleHyphens(prose) {
  const hits = [];
  // "word -- word" or "word--word" used as a dash. Not "--flag" and not "a--b" in URLs.
  const spaced = / -- /g;
  let m;
  while ((m = spaced.exec(prose)) !== null) {
    hits.push({ index: m.index, text: m[0] });
  }
  const glued = /(?<=[A-Za-z])--(?=[A-Za-z])/g;
  while ((m = glued.exec(prose)) !== null) {
    hits.push({ index: m.index, text: m[0] });
  }
  return hits;
}

export function findBannedWords(prose) {
  const hits = [];
  BANNED.lastIndex = 0;
  let m;
  while ((m = BANNED.exec(prose)) !== null) {
    hits.push({ word: m[0], index: m.index });
  }
  return hits;
}

export function sentences(prose) {
  // Drop headings and table rows for sentence length.
  const lines = prose
    .split('\n')
    .filter((line) => {
      const t = line.trim();
      if (!t) return false;
      if (/^#{1,6}\s/.test(t)) return false;
      if (/^\|/.test(t)) return false;
      if (/^[-*] /.test(t)) return false;
      if (/^>/.test(t)) return false;
      if (/^<!--/.test(t)) return false;
      return true;
    })
    .join(' ');
  return lines
    .split(/(?<=[.!?])\s+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0 && /[A-Za-z]/.test(s));
}

export function wordCount(sentence) {
  return sentence.split(/\s+/).filter(Boolean).length;
}

export function hasAudienceBlock(source) {
  return (
    /\*\*Who this is for:\*\*/i.test(source) &&
    /\*\*When you finish:\*\*/i.test(source)
  );
}

export function hasLastReviewed(source) {
  return (
    /\*\*Last reviewed:\*\*\s*\d{4}-\d{2}-\d{2}/i.test(source) &&
    /\*\*Engine:\*\*/i.test(source)
  );
}

export function firstTermUseDefined(source, term) {
  const prose = stripCode(stripNoise(source));
  const match = prose.match(term.re);
  if (!match) return { used: false, ok: true };
  const idx = match.index ?? 0;
  // Look at from start through ~200 chars after first hit for a definition.
  const window = prose.slice(0, Math.min(prose.length, idx + 200));
  // Also allow definition before first use on the same page (lede).
  const early = prose.slice(0, Math.min(prose.length, idx + 200));
  if (term.defined.test(early) || term.defined.test(window)) {
    return { used: true, ok: true };
  }
  // Glossary pages define via heading.
  if (/^#\s+Glossary/m.test(source) && new RegExp(`^#+\\s+${term.id}\\b`, 'mi').test(source)) {
    return { used: true, ok: true };
  }
  return { used: true, ok: false };
}

export function findEllipsisPlaceholders(source) {
  const hits = [];
  const fence = /```([a-zA-Z0-9_-]*)[^\n]*\n([\s\S]*?)```/g;
  let m;
  while ((m = fence.exec(source)) !== null) {
    const lang = (m[1] || '').toLowerCase();
    const body = m[2];
    if (!/^(bash|sh|shell|javascript|js|typescript|ts|python|py)$/.test(lang)) {
      continue;
    }
    if (/\.\.\./.test(body) || /\bTODO\b/.test(body) || /\byour[_-]?code\b/i.test(body)) {
      hits.push({ lang, excerpt: body.trim().slice(0, 80) });
    }
  }
  return hits;
}

export function findPassiveHedges(prose) {
  // Soft signal only: common "is/are/was/were Xed by" and "is resolved".
  const hits = [];
  const re =
    /\b(?:is|are|was|were|be|been)\s+(?:resolved|calculated|returned|rejected|validated|processed|incremented|assembled)\b/gi;
  let m;
  while ((m = re.exec(prose)) !== null) {
    hits.push({ text: m[0], index: m.index });
  }
  return hits;
}

function collectMarkdownFiles(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      collectMarkdownFiles(full, acc);
    } else if (name.endsWith('.md') || name.endsWith('.mdc')) {
      acc.push(full);
    }
  }
  return acc;
}

function collectDocsPages(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      collectDocsPages(full, acc);
    } else if (name.endsWith('.astro') && !name.startsWith('[')) {
      acc.push(full);
    }
  }
  return acc;
}

export function lintSource(rel, source, opts = {}) {
  const requireAudience = opts.requireAudience !== false;
  const requireReviewed = opts.requireReviewed !== false;
  const requireTerms = opts.requireTerms !== false;
  const errors = [];
  const warnings = [];

  const prose = stripCode(stripNoise(source));

  for (const hit of findEmDashes(prose)) {
    errors.push(`${rel}: em/en dash at offset ${hit.index} (use a full stop, comma, or colon)`);
  }
  for (const hit of findPunctuationDoubleHyphens(prose)) {
    errors.push(
      `${rel}: double hyphen used as punctuation near offset ${hit.index} (flags in code are fine)`,
    );
  }
  for (const hit of findBannedWords(prose)) {
    errors.push(`${rel}: banned word "${hit.word}" at offset ${hit.index}`);
  }
  for (const hit of findEllipsisPlaceholders(source)) {
    errors.push(
      `${rel}: incomplete ${hit.lang} example (ellipsis or placeholder): ${hit.excerpt}`,
    );
  }

  for (const s of sentences(prose)) {
    const n = wordCount(s);
    if (n > WARN_WORDS) {
      const msg = `${rel}: sentence is ${n} words (warn >${WARN_WORDS}, prefer ≤${TARGET_WORDS}): ${s.slice(0, 100)}…`;
      if (STRICT_SENTENCES) errors.push(msg);
      else warnings.push(msg);
    }
  }

  if (requireAudience && !hasAudienceBlock(source)) {
    errors.push(
      `${rel}: missing opening "**Who this is for:**" / "**When you finish:**" block`,
    );
  }
  if (requireReviewed && !hasLastReviewed(source)) {
    errors.push(`${rel}: missing "**Last reviewed:** YYYY-MM-DD" and "**Engine:**"`);
  }

  if (requireTerms) {
    for (const term of TERMS) {
      const result = firstTermUseDefined(source, term);
      if (result.used && !result.ok) {
        errors.push(
          `${rel}: first use of "${term.id}" is not defined on this page (search landing = first page)`,
        );
      }
    }
  }

  for (const hit of findPassiveHedges(prose)) {
    warnings.push(
      `${rel}: prefer active voice near "${hit.text}" (offset ${hit.index})`,
    );
  }

  return { errors, warnings };
}

function isExempt(rel) {
  // Vendor / generated / seed docs that are not reader-facing product docs.
  if (rel.startsWith('crates/')) return true;
  if (rel.startsWith('packages/')) return true;
  if (rel.startsWith('tools/') && !rel.startsWith('tools/conformance/src/methodology.md')) {
    return true;
  }
  if (rel.includes('agent-transcripts')) return true;
  if (rel === 'CHANGELOG.md') return true;
  if (rel.endsWith('LICENSE') || rel.endsWith('LICENSE.md')) return true;
  if (rel.startsWith('.cursor/')) return true;
  return false;
}

function requiresProductDocFrontMatter(rel) {
  if (rel === 'README.md') return true;
  if (rel === 'ARCHITECTURE.md') return true;
  if (rel === 'CONTRIBUTING.md') return true;
  if (rel === 'SECURITY.md') return true;
  if (rel === 'KILL-TEST.md') return false;
  if (rel === 'CONFORMANCE.md') return true;
  if (rel.startsWith('docs/')) {
    if (rel === 'docs/SECRETS.md') return false;
    if (rel === 'docs/FIVE-MINUTE-TEST.md') return false;
    if (rel === 'docs/diagrams/_include.md') return false;
    return true;
  }
  return false;
}

function main() {
  const files = [
    ...collectMarkdownFiles(ROOT),
  ]
    .map((f) => path.relative(ROOT, f))
    .filter((rel) => !isExempt(rel))
    .sort();

  // Astro docs pages that contain reader prose (checked for banned words / dashes).
  const pages = collectDocsPages(path.join(ROOT, 'web/site/src/pages'))
    .map((f) => path.relative(ROOT, f))
    .sort();

  let errors = [];
  let warnings = [];

  for (const rel of files) {
    const source = readFileSync(path.join(ROOT, rel), 'utf8');
    const product = requiresProductDocFrontMatter(rel);
    const result = lintSource(rel, source, {
      requireAudience: product && rel !== 'CONFORMANCE.md',
      requireReviewed: product && rel !== 'CONFORMANCE.md',
      requireTerms: product && rel !== 'CONFORMANCE.md',
    });
    errors = errors.concat(result.errors);
    warnings = warnings.concat(result.warnings);
  }

  for (const rel of pages) {
    const source = readFileSync(path.join(ROOT, rel), 'utf8');
    // Astro: only lint prose-ish strings; skip full frontmatter audience requirement.
    const result = lintSource(rel, source, {
      requireAudience: false,
      requireReviewed: false,
      requireTerms: false,
    });
    errors = errors.concat(result.errors);
    warnings = warnings.concat(result.warnings);
  }

  for (const w of warnings) {
    console.warn(`WARN ${w}`);
  }
  for (const e of errors) {
    console.error(`FAIL ${e}`);
  }

  if (errors.length) {
    console.error(`docs-lint: ${errors.length} error(s), ${warnings.length} warning(s)`);
    process.exit(1);
  }
  console.log(
    `docs-lint: clean (${files.length} markdown, ${pages.length} pages, ${warnings.length} warning(s))`,
  );
}

const isDirect =
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isDirect) {
  main();
}
