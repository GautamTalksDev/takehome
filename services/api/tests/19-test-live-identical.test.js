import assert from 'node:assert/strict';
import { test } from 'node:test';
import { KEY_HEX_CHARS } from '../src/keys.js';
import { call, createWorld, ON_WEEKLY } from './helpers.js';

test('19. test and live keys share the np_ prefixes', () => {
  const world = createWorld();
  assert.match(world.testKey, new RegExp(`^np_test_[0-9a-f]{${KEY_HEX_CHARS}}$`));
  assert.match(world.liveKey, new RegExp(`^np_live_[0-9a-f]{${KEY_HEX_CHARS}}$`));
});

test('19. a test key and a live key produce identical engine output', async () => {
  const world = createWorld();
  const testRes = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  const liveRes = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(testRes.status, 200);
  assert.equal(liveRes.status, 200);
  assert.equal(testRes.json.employee.net_pay, liveRes.json.employee.net_pay);
  assert.equal(testRes.json.employee.federal_tax, liveRes.json.employee.federal_tax);
  assert.equal(testRes.json.rule_set_version, liveRes.json.rule_set_version);
  assert.equal(testRes.json.engine_build_sha256, liveRes.json.engine_build_sha256);
  assert.deepEqual(testRes.json.breakdown, liveRes.json.breakdown);
});
