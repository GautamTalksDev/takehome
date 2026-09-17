import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ON_WEEKLY } from './helpers.js';

test('12. as_of before coverage is date_out_of_range, not nearest-match', async () => {
  const { status, json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    as_of: '2025-12-31',
  });
  assert.equal(status, 400);
  assert.equal(json.error.code, 'date_out_of_range');
  assert.match(json.error.message, /2025-12-31/);
  assert.match(json.error.message, /2026-01-01/);
  assert.equal(json.employee, undefined);
  assert.equal(json.rule_set_version, null);
});

test('12. coverage start is included, not skipped', async () => {
  const { status, json } = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    as_of: '2026-01-01',
  });
  assert.equal(status, 200);
  assert.equal(json.rule_set_version, '2026-01-01');
  assert.equal(typeof json.employee.net_pay, 'string');
});

test('12. unknown rule-set version is 404, not nearest-match', async () => {
  const { status, json } = await call('GET', '/v1/rules/1999-01-01');
  assert.equal(status, 404);
  assert.equal(json.error.code, 'not_found');
  assert.match(json.error.message, /1999-01-01/);
  assert.equal(json.employee, undefined);
});
