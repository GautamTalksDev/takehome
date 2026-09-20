import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY } from './helpers.js';

test('23. live keys hard-stop at the plan limit and do not run the extra calculation', async () => {
  const world = createWorld('developer');
  const limit = 100_000;
  world.store.setUsage(world.account.id, '2026-09-01', limit);
  const { status, json, response } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 402);
  assert.equal(json.error.code, 'plan_limit');
  assert.match(json.error.message, /100000|100,000/);
  assert.match(json.error.docs, /pricing/);
  assert.equal(json.employee, undefined);
  assert.equal(response.headers.get('x-usage-remaining'), '0');
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), limit);
});

test('23. a 500-item batch that exceeds remaining is not surprise-billed', async () => {
  const world = createWorld();
  world.store.setUsage(world.account.id, '2026-09-01', 99_600);
  const requests = Array.from({ length: 500 }, () => ON_WEEKLY);
  const { status, json } = await call(
    'POST',
    '/v1/deductions/batch',
    { requests },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 402);
  assert.equal(json.error.code, 'plan_limit');
  assert.match(json.error.message, /500/);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 99_600);
});

test('23. test keys never hard-stop', async () => {
  const world = createWorld();
  world.store.setUsage(world.account.id, '2026-09-01', 1000);
  const { status } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 200);
});
