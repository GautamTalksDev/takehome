import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ON_WEEKLY, newStore } from './helpers.js';
import { BASE_ENV } from './helpers.js';

test('26. signup with ECHO_VERIFY_URL, first authenticated call is 800.79', async () => {
  const store = newStore();
  const mailbox = [];
  const env = {
    ...BASE_ENV,
    STORE: store,
    MAILBOX: mailbox,
    ECHO_VERIFY_URL: '1',
  };
  const world = { env, testKey: null };

  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'five-minute@example.com' },
    {},
    world,
  );
  assert.equal(signed.status, 200);
  assert.match(signed.json.verify_url, /\/signup\/verify\/\?token=/);
  const token = new URL(signed.json.verify_url).searchParams.get('token');
  assert.ok(token);

  const verified = await call(
    'GET',
    `/v1/signup/verify?token=${token}`,
    undefined,
    {},
    world,
  );
  assert.equal(verified.status, 200);
  assert.match(verified.json.test_key, /^np_test_/);

  const first = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${verified.json.test_key}` },
    world,
  );
  assert.equal(first.status, 200);
  assert.equal(first.json.employee.net_pay, '800.79');
});
