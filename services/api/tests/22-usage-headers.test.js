import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY } from './helpers.js';

test('22. live deductions responses carry limit, remaining, reset', async () => {
  const world = createWorld();
  const { status, response } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 200);
  assert.equal(response.headers.get('x-usage-limit'), '1000');
  assert.equal(response.headers.get('x-usage-remaining'), '999');
  assert.equal(response.headers.get('x-usage-reset'), '2026-10-01T00:00:00Z');
});

test('22. test keys report unlimited usage headers', async () => {
  const world = createWorld();
  const { status, response } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 200);
  assert.equal(response.headers.get('x-usage-limit'), 'unlimited');
  assert.equal(response.headers.get('x-usage-remaining'), 'unlimited');
  assert.ok(response.headers.get('x-usage-reset'));
});
