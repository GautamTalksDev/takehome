/**
 * Astro HTML minify collapses newlines between text and tags.
 * Fail if a word character sits flush against <code or <a.
 */
import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');
const GLUE = /([A-Za-z0-9.,;:])(<(?:code|a)\b)/g;

function walkHtml(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    const st = statSync(full);
    if (st.isDirectory()) walkHtml(full, out);
    else if (name.endsWith('.html')) out.push(full);
  }
  return out;
}

test('emitted HTML keeps a space before inline code and links', () => {
  assert.ok(statSync(DIST).isDirectory(), `missing ${DIST}; run npm run build`);
  const bad = [];
  for (const file of walkHtml(DIST)) {
    const html = readFileSync(file, 'utf8');
    for (const match of html.matchAll(GLUE)) {
      const i = match.index ?? 0;
      const ctx = html.slice(Math.max(0, i - 24), i + 48).replace(/\s+/g, ' ');
      bad.push(`${path.relative(DIST, file)}: …${ctx}…`);
    }
  }
  assert.equal(bad.length, 0, `adjacent-token collapses:\n${bad.join('\n')}`);
});
