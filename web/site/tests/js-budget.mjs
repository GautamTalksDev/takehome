/**
 * JS excluding WASM must stay under 50KB (spec §13.3).
 */
import assert from 'node:assert/strict';
import { readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const DIST = path.join(ROOT, 'dist');
const LIMIT = 50 * 1024;

function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) {
      walk(full, acc);
    } else {
      acc.push(full);
    }
  }
  return acc;
}

const files = walk(DIST).filter((f) => f.endsWith('.js'));
let total = 0;
for (const file of files) {
  const bytes = statSync(file).size;
  total += bytes;
  console.log(`${path.relative(DIST, file)}: ${bytes}`);
}
console.log(`js excluding wasm: ${total} bytes (limit ${LIMIT})`);
assert.ok(
  total <= LIMIT,
  `JS budget exceeded: ${total} bytes > ${LIMIT}. WASM is excluded; other JS is not.`,
);
assert.ok(files.length > 0, 'no JS files in dist; calculator script missing');
