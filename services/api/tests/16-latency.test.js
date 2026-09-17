import assert from 'node:assert/strict';
import { test } from 'node:test';
import { performance } from 'node:perf_hooks';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { ENV, fixtures, ON_WEEKLY, request } from './helpers.js';

test('16. POST /v1/deductions p99 is under 30ms excluding network', async () => {
  const req = request('POST', '/v1/deductions', ON_WEEKLY, {
    authorization: `Bearer ${fixtures.testKey}`,
  });
  for (let i = 0; i < 25; i += 1) {
    await handle(req.clone(), ENV, nodeEngine);
  }
  const samples = [];
  for (let i = 0; i < 200; i += 1) {
    const start = performance.now();
    const response = await handle(req.clone(), ENV, nodeEngine);
    samples.push(performance.now() - start);
    assert.equal(response.status, 200);
  }
  samples.sort((a, b) => a - b);
  const p99 = samples[197];
  assert.ok(
    p99 < 30,
    `p99 ${p99}ms (n=200 local handle, no network). Floor is 30ms.`,
  );
});
