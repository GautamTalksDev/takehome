import assert from 'node:assert/strict';
import { gzipSync } from 'node:zlib';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { wasmPath } from './helpers.js';

test('gzipped WASM stays under the committed size threshold', () => {
  const here = path.dirname(fileURLToPath(import.meta.url));
  const limit = JSON.parse(
    readFileSync(path.join(here, 'fixtures/wasm-size-limit.json'), 'utf8'),
  );
  const wasm = readFileSync(wasmPath());
  const gzipped = gzipSync(wasm, { level: 9 });
  assert.ok(
    gzipped.length <= limit.gzip_bytes_max,
    `gzipped WASM is ${gzipped.length} bytes; committed max is ${limit.gzip_bytes_max} (recorded ${limit.recorded_gzip_bytes})`,
  );
  // Printed so CI logs quote the number.
  console.log(
    `gzipped wasm: ${gzipped.length} bytes (recorded ${limit.recorded_gzip_bytes}, max ${limit.gzip_bytes_max})`,
  );
});
