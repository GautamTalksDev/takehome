import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call } from './helpers.js';

test('17. Quebec returns JurisdictionNotSupported, never T2=0', async () => {
  const { status, json } = await call('POST', '/v1/deductions', {
    as_of: '2026-01-15',
    province: 'QC',
    pay_period: 52,
    gross_pay: '1000.00',
  });
  assert.equal(status, 422);
  assert.equal(json.error.code, 'jurisdiction_not_supported');
  assert.match(json.error.message, /Quebec/);
  assert.equal(json.employee, undefined);
  assert.equal(json.provincial_tax, undefined);
});

test('17. GET /v1/jurisdictions lists Quebec as unsupported', async () => {
  const { status, json } = await call('GET', '/v1/jurisdictions');
  assert.equal(status, 200);
  const qc = json.jurisdictions.find((row) => row.code === 'QC');
  assert.ok(qc);
  assert.equal(qc.supported, false);
  assert.match(qc.note, /Quebec/);
  assert.match(qc.note, /T2 = 0/);
});
