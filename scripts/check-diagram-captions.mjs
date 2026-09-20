#!/usr/bin/env node
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const captions = JSON.parse(
  readFileSync(path.join(ROOT, 'docs/diagrams/captions.json'), 'utf8'),
);
const outDir = path.join(ROOT, 'web/site/public/diagrams');
let failed = 0;
for (const [id, meta] of Object.entries(captions)) {
  if (!meta.caption || meta.caption.length < 40) {
    console.error(`FAIL ${id}: caption too short or missing`);
    failed += 1;
  }
  const svg = path.join(outDir, `${id}.svg`);
  if (!existsSync(svg)) {
    console.error(`FAIL ${id}: missing ${svg}`);
    failed += 1;
  }
}
for (const name of readdirSync(path.join(ROOT, 'docs/diagrams'))) {
  if (!name.endsWith('.mmd')) continue;
  const id = name.replace(/\.mmd$/, '');
  if (!captions[id]) {
    console.error(`FAIL ${id}: .mmd has no captions.json entry`);
    failed += 1;
  }
}
if (failed) process.exit(1);
console.log('diagram captions: ok');
