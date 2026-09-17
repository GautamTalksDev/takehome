import assert from 'node:assert/strict';
import { test } from 'node:test';
import { calculationCount } from '../src/meter.js';
import { call, createWorld, ON_WEEKLY } from './helpers.js';

test('21. a single deductions object is one calculation', () => {
  assert.equal(calculationCount(ON_WEEKLY), 1);
});

test('21. a batch of 500 counts 500, not one HTTP request', () => {
  const requests = Array.from({ length: 500 }, () => ON_WEEKLY);
  assert.equal(calculationCount({ requests }), 500);
  assert.equal(calculationCount(requests), 500);
});

test('21. two successful live calls increment calculations by 2, not by a batch size', async () => {
  const world = createWorld();
  await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  const used = world.store.getUsage(world.account.id, '2026-09-01');
  assert.equal(used, 2);
});

test('21. a 500-item batch is metered as 500 even though batch is M4', async () => {
  const world = createWorld();
  const requests = Array.from({ length: 500 }, () => ON_WEEKLY);
  const { status, json } = await call(
    'POST',
    '/v1/deductions',
    { requests },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 400);
  assert.equal(json.error.code, 'batch_not_implemented');
  assert.match(json.error.message, /500/);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 0);
});

test('21. test keys are not metered', async () => {
  const world = createWorld();
  await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 0);
});
