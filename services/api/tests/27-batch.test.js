import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY } from './helpers.js';

const THRESHOLD = JSON.parse(
  readFileSync(
    path.join(path.dirname(fileURLToPath(import.meta.url)), 'batch-p99-threshold.json'),
    'utf8',
  ),
);

const JULY_WEEKLY = { ...ON_WEEKLY, as_of: '2026-07-01' };

function requests(n, at) {
  return Array.from({ length: n }, (_, i) => (at ? at(i) : { ...ON_WEEKLY }));
}

test('15. 1000 items return 1000 results in input order, each with its own rule_set_version', async () => {
  const world = createWorld();
  const body = {
    requests: requests(1000, (i) => (i % 2 === 0 ? { ...ON_WEEKLY } : { ...JULY_WEEKLY })),
  };
  const { status, json } = await call(
    'POST',
    '/v1/deductions/batch',
    body,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 200);
  assert.equal(json.results.length, 1000);
  for (let i = 0; i < 1000; i += 1) {
    const item = json.results[i];
    assert.equal(item.ok, true, `item ${i} should succeed`);
    assert.equal(
      item.response.rule_set_version,
      i % 2 === 0 ? '2026-01-01' : '2026-07-01',
      `item ${i} rule_set_version`,
    );
  }
  assert.equal(json.results[0].response.employee.net_pay, '800.79');
  assert.notEqual(
    json.results[0].response.rule_set_version,
    json.results[1].response.rule_set_version,
  );
});

test('16. item 500 invalid does not fail the batch; each result is {ok, response} or {error}', async () => {
  const world = createWorld();
  const items = requests(1000);
  items[499] = { ...ON_WEEKLY, not_a_t4127_field: '1.00' };
  const { status, json } = await call(
    'POST',
    '/v1/deductions/batch',
    { requests: items },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 200);
  assert.equal(json.results.length, 1000);
  assert.equal(json.results[0].ok, true);
  assert.equal(json.results[0].response.employee.net_pay, '800.79');
  assert.equal(json.results[498].ok, true);
  assert.equal(json.results[499].ok, undefined);
  assert.equal(json.results[499].response, undefined);
  assert.equal(typeof json.results[499].error.code, 'string');
  assert.match(json.results[499].error.code, /unknown_field/);
  assert.match(json.results[499].error.message, /not_a_t4127_field/);
  assert.match(json.results[499].error.docs, /^https:\/\//);
  assert.equal(json.results[500].ok, true);
  assert.equal(json.results[999].ok, true);
});

test('17. 1001 items are rejected with a clear error naming the 1000 limit', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/deductions/batch',
    { requests: requests(1001) },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 400);
  assert.equal(json.error.code, 'batch_too_large');
  assert.match(json.error.message, /1000/);
  assert.match(json.error.message, /1001/);
  assert.equal(json.results, undefined);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 0);
});

test('18. a 1000-item batch is metered as 1000, not 1', async () => {
  const world = createWorld();
  const { status, json, response } = await call(
    'POST',
    '/v1/deductions/batch',
    { requests: requests(1000) },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 200);
  assert.equal(json.results.length, 1000);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 1000);
  assert.equal(response.headers.get('x-usage-remaining'), '0');
});

test('19. over-quota mid-batch rejects the whole batch with 402 and does not charge', async () => {
  const world = createWorld();
  world.store.setUsage(world.account.id, '2026-09-01', 1);
  const { status, json } = await call(
    'POST',
    '/v1/deductions/batch',
    { requests: requests(1000) },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 402);
  assert.equal(json.error.code, 'plan_limit');
  assert.match(json.error.message, /1000/);
  assert.equal(json.results, undefined);
  assert.equal(json.employee, undefined);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 1);
});

test('20. the same batch twice is byte-identical', async () => {
  const world = createWorld();
  const body = {
    requests: requests(25, (i) =>
      i === 10 ? { ...ON_WEEKLY, not_a_t4127_field: '1.00' } : { ...ON_WEEKLY },
    ),
  };
  const first = await call(
    'POST',
    '/v1/deductions/batch',
    body,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  const second = await call(
    'POST',
    '/v1/deductions/batch',
    body,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(first.status, 200);
  assert.equal(second.status, 200);
  assert.equal(first.text, second.text);
});

test('21. p99 for a 1000-item batch is under the committed threshold', async () => {
  const world = createWorld();
  const body = { requests: requests(1000) };
  const headers = { authorization: `Bearer ${world.testKey}` };
  for (let i = 0; i < THRESHOLD.warmup; i += 1) {
    const warm = await call('POST', '/v1/deductions/batch', body, headers, world);
    assert.equal(warm.status, 200);
  }
  const samples = [];
  for (let i = 0; i < THRESHOLD.samples; i += 1) {
    const start = performance.now();
    const { status } = await call(
      'POST',
      '/v1/deductions/batch',
      body,
      headers,
      world,
    );
    samples.push(performance.now() - start);
    assert.equal(status, 200);
  }
  samples.sort((a, b) => a - b);
  const p99 = samples[Math.min(samples.length - 1, Math.ceil(THRESHOLD.samples * 0.99) - 1)];
  assert.ok(
    p99 < THRESHOLD.p99_ms,
    `p99 ${p99}ms (n=${THRESHOLD.samples} local handle, no network). Floor is ${THRESHOLD.p99_ms}ms.`,
  );
});
