import { writeFileSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildOpenApi } from '../src/openapi.js';

const dest = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  'openapi.json',
);
const generated = `${JSON.stringify(buildOpenApi(), null, 2)}\n`;

if (process.argv.includes('--check')) {
  let onDisk;
  try {
    onDisk = readFileSync(dest, 'utf8');
  } catch {
    console.error(`missing ${dest}; run node scripts/generate-openapi.js`);
    process.exit(1);
  }
  if (onDisk !== generated) {
    console.error('openapi.json is stale. Run node scripts/generate-openapi.js');
    process.exit(1);
  }
  console.log('openapi.json matches src/schema.js');
  process.exit(0);
}

writeFileSync(dest, generated);
console.log(`wrote ${dest}`);
