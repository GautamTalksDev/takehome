import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ON_WEEKLY } from './helpers.js';

const IDENTITY = ['rule_set_version', 'engine_version', 'engine_build_sha256'];

const PUBLIC_GETS = [
  '/v1/rules',
  '/v1/rules/2026-01-01',
  '/v1/rules/diff?from=2026-01-01&to=2026-07-01',
  '/v1/jurisdictions',
  '/v1/changes',
  '/v1/conformance',
  '/openapi.json',
];

test('13. POST /v1/deductions carries identity fields', async () => {
  const { status, json } = await call('POST', '/v1/deductions', ON_WEEKLY);
  assert.equal(status, 200);
  for (const key of IDENTITY) {
    assert.equal(typeof json[key], 'string', key);
    assert.ok(json[key].length > 0, key);
  }
  assert.match(json.engine_build_sha256, /^[0-9a-f]{64}$/);
});

test('13. every M3 GET carries identity fields', async () => {
  for (const path of PUBLIC_GETS) {
    const { status, json } = await call('GET', path);
    assert.equal(status, 200, path);
    assert.equal(typeof json.engine_version, 'string', path);
    assert.match(json.engine_build_sha256, /^[0-9a-f]{64}$/, path);
    assert.ok(
      json.rule_set_version === null || typeof json.rule_set_version === 'string',
      path,
    );
  }
});

test('13. error responses still carry engine identity', async () => {
  const { json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    typo_gross: '1.00',
  });
  assert.equal(typeof json.engine_version, 'string');
  assert.match(json.engine_build_sha256, /^[0-9a-f]{64}$/);
});
