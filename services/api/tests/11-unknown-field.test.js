import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ON_WEEKLY } from './helpers.js';

test('11. unknown field is rejected and named, never ignored', async () => {
  const { status, json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    typo_gross: '1.00',
  });
  assert.equal(status, 400);
  assert.equal(json.error.code, 'unknown_field');
  assert.match(json.error.message, /typo_gross/);
  assert.equal(json.employee, undefined);
});

test('11. a province key typo is named, not treated as Ontario', async () => {
  const { status, json } = await call('POST', '/v1/deductions', {
    as_of: '2026-01-15',
    provance: 'ON',
    pay_period: 52,
    gross_pay: '1000.00',
  });
  assert.equal(status, 400);
  assert.match(json.error.message, /provance/);
  assert.equal(json.employee, undefined);
});
