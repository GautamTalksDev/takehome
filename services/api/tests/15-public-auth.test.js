import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, ENV, request } from './helpers.js';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { buildOpenApi } from '../src/openapi.js';

const PUBLIC = [
  '/v1/rules',
  '/v1/rules/diff',
  '/v1/changes',
  '/v1/changes.rss',
  '/v1/conformance',
  '/openapi.json',
];

test('15. public goods do not require auth', async () => {
  for (const path of PUBLIC) {
    const url =
      path === '/v1/rules/diff'
        ? '/v1/rules/diff?from=2026-01-01&to=2026-07-01'
        : path;
    const { status, response } = await call('GET', url);
    assert.equal(status, 200, path);
    assert.equal(response.headers.get('www-authenticate'), null, path);
  }
});

test('15. OpenAPI lists those paths with no security', async () => {
  const spec = buildOpenApi();
  for (const path of PUBLIC) {
    const op = spec.paths[path].get;
    assert.ok(op, path);
    const security = op.security ?? spec.security ?? [];
    assert.deepEqual(security, [], path);
  }
});

test('15. IP rate limit returns a structured 429, not an auth challenge', async () => {
  const tight = { ...ENV, RATE_LIMIT_MAX: '2', RATE_LIMIT_WINDOW_MS: '60000' };
  const ip = { 'cf-connecting-ip': '203.0.113.9' };
  await handle(request('GET', '/v1/rules', undefined, ip), tight, nodeEngine);
  await handle(request('GET', '/v1/rules', undefined, ip), tight, nodeEngine);
  const third = await handle(
    request('GET', '/v1/rules', undefined, ip),
    tight,
    nodeEngine,
  );
  assert.equal(third.status, 429);
  assert.equal(third.headers.get('www-authenticate'), null);
  const json = await third.json();
  assert.equal(json.error.code, 'rate_limited');
  assert.match(json.error.docs, /^https:\/\//);
});
