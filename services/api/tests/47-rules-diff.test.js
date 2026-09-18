import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call } from './helpers.js';

test('47. GET /v1/rules/diff Jan→Jul 2026 names exactly BC, NL, and PE', async () => {
  const { status, json } = await call(
    'GET',
    '/v1/rules/diff?from=2026-01-01&to=2026-07-01',
  );
  assert.equal(status, 200, JSON.stringify(json));
  assert.deepEqual(json.changed, ['BC', 'NL', 'PE']);
  assert.equal(json.from, '2026-01-01');
  assert.equal(json.to, '2026-07-01');
  assert.deepEqual(Object.keys(json.fields).sort(), ['BC', 'NL', 'PE']);
  assert.deepEqual(json.cpp, []);
  assert.deepEqual(json.ei, []);
  assert.deepEqual(json.qpip, []);
});

test('47. GET /v1/rules/diff is not captured as /v1/rules/:version', async () => {
  const missing = await call('GET', '/v1/rules/diff');
  assert.equal(missing.status, 400);
  assert.equal(missing.json.error.code, 'invalid_request');
  const unknown = await call('GET', '/v1/rules/2099-01-01');
  assert.equal(unknown.status, 404);
});
