import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY } from './helpers.js';
import { hashKey } from '../src/keys.js';

test('20. missing bearer is 401', async () => {
  const { status, json, response } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: null },
  );
  assert.equal(status, 401);
  assert.equal(json.error.code, 'unauthorized');
  assert.equal(response.headers.get('www-authenticate'), 'Bearer');
});

test('20. keys are stored hashed and the secret is not in D1', async () => {
  const world = createWorld();
  const dumped = world.store.dump();
  assert.equal(dumped.api_keys.length, 2);
  const liveHash = await hashKey(world.liveKey);
  const found = dumped.api_keys.find((row) => row.hash === liveHash);
  assert.ok(found);
  assert.equal(found.hash.length, 64);
  assert.equal(found.secret, undefined);
  assert.equal(found.plaintext, undefined);
  assert.ok(!JSON.stringify(dumped).includes(world.liveKey));
  assert.ok(!JSON.stringify(dumped).includes(world.testKey.slice('np_test_'.length)));
});

test('20. a random bearer does not recover a key', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: 'Bearer np_live_ffffffffffffffffffffffffffffffffffffffffffffffff' },
    world,
  );
  assert.equal(status, 401);
  assert.equal(json.error.code, 'unauthorized');
});
