import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import {
  loadEngine,
  loadGridSample,
  loadM1Vectors,
  nativeCalculateBatch,
} from './helpers.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));

function stripEngineSha(jsonText) {
  const parsed = JSON.parse(jsonText);
  delete parsed.engine_build_sha256;
  return JSON.stringify(parsed);
}

test('M1 PDOC vectors: WASM output is byte-identical to native', async () => {
  const engine = await loadEngine();
  const vectors = loadM1Vectors();
  const requests = vectors.map((v) => v.request);
  const native = nativeCalculateBatch(requests);
  assert.equal(native.length, 20);
  for (let i = 0; i < vectors.length; i += 1) {
    const wasm = engine.calculate(JSON.stringify(requests[i]));
    assert.equal(
      wasm,
      native[i],
      `${vectors[i].id}: WASM and native disagree`,
    );
    const parsed = JSON.parse(wasm);
    assert.equal(typeof parsed.employee.net_pay, 'string', vectors[i].id);
    assert.equal(parsed.employee.net_pay, vectors[i].expected.net_pay, vectors[i].id);
  }
});

test('500 grid cases across thirteen jurisdictions: WASM equals native', async () => {
  const engine = await loadEngine();
  const sample = loadGridSample();
  assert.equal(sample.jurisdictions.length, 13);
  const native = nativeCalculateBatch(sample.requests);
  assert.equal(native.length, 500);
  const seen = new Set();
  for (let i = 0; i < sample.requests.length; i += 1) {
    const requestJson = JSON.stringify(sample.requests[i]);
    const wasm = engine.calculate(requestJson);
    assert.equal(wasm, native[i], `grid[${i}] ${sample.ids[i]}`);
    const parsed = JSON.parse(wasm);
    assert.equal(typeof parsed.error, 'undefined', sample.ids[i]);
    seen.add(sample.requests[i].province);
  }
  assert.equal(seen.size, 13, `expected 13 jurisdictions, got ${[...seen]}`);
});

test('engineBuildSha() matches native engine_build_sha256', async () => {
  const engine = await loadEngine();
  const wasmSha = engine.engineBuildSha();
  const native = nativeCalculateBatch([
    {
      as_of: '2026-01-15',
      province: 'ON',
      pay_period: 52,
      gross_pay: '1000.00',
    },
  ]);
  const nativeSha = JSON.parse(native[0]).engine_build_sha256;
  assert.equal(wasmSha, nativeSha);
  assert.equal(wasmSha.length, 64);
  assert.match(wasmSha, /^[0-9a-f]{64}$/);
});

test('engine_build_sha256 changed; M1 vector bodies stay byte-identical to the pre-rename goldens', async () => {
  const recorded = JSON.parse(
    readFileSync(path.join(HERE, 'fixtures/pre-rename-engine.json'), 'utf8'),
  );
  const engine = await loadEngine();
  const wasmSha = engine.engineBuildSha();
  const native = nativeCalculateBatch(recorded.requests);
  assert.equal(native.length, recorded.requests.length);
  const nativeSha = JSON.parse(native[0]).engine_build_sha256;
  assert.equal(wasmSha, nativeSha);
  assert.notEqual(
    wasmSha,
    recorded.engine_build_sha256,
    'source changed, so engine_build_sha256 must change',
  );
  for (let i = 0; i < recorded.requests.length; i += 1) {
    const wasm = engine.calculate(JSON.stringify(recorded.requests[i]));
    assert.equal(
      stripEngineSha(wasm),
      recorded.stripped[i],
      `wasm M1[${i}] cents changed`,
    );
    assert.equal(
      stripEngineSha(native[i]),
      recorded.stripped[i],
      `native M1[${i}] cents changed`,
    );
  }
});
