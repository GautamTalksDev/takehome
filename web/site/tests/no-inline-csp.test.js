/**
 * Build-time CSP guard. Emitted HTML must not contain inline <script>
 * bodies or HTML on* handlers. script-src 'self' already blocks them at
 * runtime; this fails the build where it is cheap.
 */
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');

const INLINE_SCRIPT =
  /<script(?![^>]*\bsrc\s*=)(?![^>]*\btype\s*=\s*["']application\/ld\+json["'])[^>]*>[\s\S]*?<\/script>/gi;
// Match onsubmit=, onclick=, etc.
const ON_HANDLER = /\s(on[a-z]+)\s*=\s*(["'])/gi;

function walkHtml(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) {
      walkHtml(full, out);
    } else if (name.endsWith('.html')) {
      out.push(full);
    }
  }
  return out;
}

function violations(html) {
  const found = [];
  for (const match of html.matchAll(INLINE_SCRIPT)) {
    found.push({ kind: 'inline-script', snippet: match[0].slice(0, 120) });
  }
  for (const match of html.matchAll(ON_HANDLER)) {
    found.push({ kind: 'on-handler', snippet: match[0].slice(0, 80) });
  }
  return found;
}

test('emitted HTML has no inline script bodies or on* handlers', () => {
  assert.ok(statSync(DIST).isDirectory(), `missing ${DIST}; run npm run build`);
  const files = walkHtml(DIST);
  assert.ok(files.length > 0, 'dist has no HTML');
  const bad = [];
  for (const file of files) {
    const html = readFileSync(file, 'utf8');
    for (const hit of violations(html)) {
      bad.push({ file: path.relative(DIST, file), ...hit });
    }
  }
  assert.equal(
    bad.length,
    0,
    `CSP-unsafe HTML in dist:\n${bad
      .map((row) => `  ${row.file}: ${row.kind}: ${row.snippet.replace(/\s+/g, ' ')}`)
      .join('\n')}`,
  );
});

test('Sources sections do not embed serialized citation objects', () => {
  assert.ok(statSync(DIST).isDirectory(), `missing ${DIST}; run npm run build`);
  const files = walkHtml(DIST).filter((file) =>
    /Sources/.test(readFileSync(file, 'utf8')),
  );
  assert.ok(files.length > 0, 'expected calculator pages with Sources');
  const bad = [];
  for (const file of files) {
    const html = readFileSync(file, 'utf8');
    const start = html.search(/<h2[^>]*>Sources<\/h2>/i);
    if (start < 0) continue;
    const slice = html.slice(start, start + 800);
    if (
      slice.includes('{') ||
      slice.includes('"what":') ||
      slice.includes('factor-index')
    ) {
      bad.push(path.relative(DIST, file));
    }
  }
  assert.equal(
    bad.length,
    0,
    `Sources still dumps citation JSON:\n${bad.map((row) => `  ${row}`).join('\n')}`,
  );
});
