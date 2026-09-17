import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ON_WEEKLY } from './helpers.js';

test('14. errors are code + sentence + docs link', async () => {
  const { json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    typo_gross: '1.00',
  });
  assert.equal(typeof json.error.code, 'string');
  assert.ok(json.error.code.length > 0);
  assert.equal(typeof json.error.message, 'string');
  assert.match(json.error.message, / /);
  assert.match(json.error.docs, /^https:\/\//);
});

test('14. date_out_of_range points at T4127 docs', async () => {
  const { json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    as_of: '2025-12-31',
  });
  assert.equal(json.error.code, 'date_out_of_range');
  assert.match(json.error.docs, /^https:\/\//);
  assert.match(json.error.message, /coverage/);
});
