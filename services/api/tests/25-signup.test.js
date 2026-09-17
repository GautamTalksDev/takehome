import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY, request } from './helpers.js';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { MemoryStore } from '../src/store-memory.js';
import { BASE_ENV } from './helpers.js';

test('25. signup is email, verify, keys, first call', async () => {
  const store = new MemoryStore();
  const mailbox = [];
  const env = { ...BASE_ENV, STORE: store, MAILBOX: mailbox };
  const world = { env, testKey: null };

  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'ada@example.com' },
    {},
    world,
  );
  assert.equal(signed.status, 200);
  assert.equal(signed.json.test_key, undefined);
  assert.equal(signed.json.live_key, undefined);
  assert.equal(signed.json.verify_url, undefined);
  assert.equal(mailbox.length, 1);
  assert.equal(mailbox[0].to, 'ada@example.com');
  assert.ok(mailbox[0].token);
  assert.doesNotMatch(JSON.stringify(store.dump()), new RegExp(mailbox[0].token));

  const verified = await call(
    'GET',
    `/v1/signup/verify?token=${mailbox[0].token}`,
    undefined,
    {},
    world,
  );
  assert.equal(verified.status, 200);
  assert.match(verified.json.test_key, /^np_test_/);
  assert.match(verified.json.live_key, /^np_live_/);

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

test('25. signup copy has no sales call or onboarding', async () => {
  const store = new MemoryStore();
  const env = { ...BASE_ENV, STORE: store, MAILBOX: [] };
  const res = await handle(
    request('POST', '/v1/signup', { email: 'bob@example.com' }),
    env,
    nodeEngine,
  );
  const json = await res.json();
  assert.doesNotMatch(json.message, /sales/i);
  assert.doesNotMatch(json.message, /onboarding/i);
  assert.doesNotMatch(json.message, /demo/i);
});
