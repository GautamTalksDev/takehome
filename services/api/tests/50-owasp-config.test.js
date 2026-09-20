import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY, request } from './helpers.js';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { isPublicCorsPath } from '../src/cors.js';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const WRANGLER = readFileSync(path.join(ROOT, '../wrangler.toml'), 'utf8');

const LEAK = /(?:^|[^a-z])(?:wrangler|workerd|panicked)(?:[^a-z]|$)|\/home\/|\/usr\/|\\\\Users\\\\|SELECT\s+[\w*]+\s+FROM\s+/i;

function assertCleanError(json, label) {
  assert.equal(typeof json.error, 'object', label);
  assert.equal(typeof json.error.code, 'string', label);
  assert.ok(json.error.code.length > 0, label);
  assert.equal(typeof json.error.message, 'string', label);
  assert.match(json.error.docs, /^https:\/\//, label);
  assert.deepEqual(
    Object.keys(json.error).sort(),
    ['code', 'docs', 'message'],
    label,
  );
  const dumped = JSON.stringify(json);
  assert.equal(LEAK.test(dumped), false, `${label}: ${dumped}`);
}

const PUBLIC_CORS = [
  '/v1/rules',
  '/v1/rules/2026-07-01',
  '/v1/rules/diff?from=2026-01-01&to=2026-07-01',
  '/v1/changes',
  '/v1/conformance',
  '/openapi.json',
];

const CLOSED_CORS = [
  ['POST', '/v1/deductions', ON_WEEKLY],
  ['POST', '/v1/deductions/batch', { requests: [ON_WEEKLY] }],
  ['POST', '/v1/deductions/year', { ...ON_WEEKLY, as_of: '2026-01-01', pay_period: 26 }],
  ['GET', '/v1/webhooks', undefined],
  ['GET', '/v1/keys', undefined],
  ['POST', '/v1/keys/rotate', { kind: 'test' }],
  ['POST', '/v1/keys/revoke', { kind: 'test' }],
  ['POST', '/v1/signup', { email: 'closed-cors@example.com' }],
  ['GET', '/health', undefined],
];

test('50. public unauthenticated endpoints are CORS-open; keyed routes are not', async () => {
  for (const path of PUBLIC_CORS) {
    const { response, status } = await call('GET', path);
    assert.equal(status, 200, path);
    assert.equal(
      response.headers.get('access-control-allow-origin'),
      '*',
      path,
    );
  }
  const world = createWorld();
  for (const [method, path, body] of CLOSED_CORS) {
    const { response } = await call(
      method,
      path,
      body,
      { authorization: `Bearer ${world.testKey}` },
      world,
    );
    assert.equal(
      response.headers.get('access-control-allow-origin'),
      null,
      path,
    );
  }
});

test('50. OPTIONS preflight is * only on public CORS paths', async () => {
  const publicOpt = await handle(
    request('OPTIONS', '/v1/rules', undefined, {
      origin: 'https://evil.example',
    }),
    createWorld().env,
    nodeEngine,
  );
  assert.equal(publicOpt.status, 204);
  assert.equal(publicOpt.headers.get('access-control-allow-origin'), '*');

  const keyedOpt = await handle(
    request('OPTIONS', '/v1/deductions', undefined, {
      origin: 'https://evil.example',
      'access-control-request-method': 'POST',
    }),
    createWorld().env,
    nodeEngine,
  );
  assert.equal(keyedOpt.headers.get('access-control-allow-origin'), null);
  assert.equal(isPublicCorsPath('/v1/deductions'), false);
  assert.equal(isPublicCorsPath('/v1/rules'), true);
  assert.equal(isPublicCorsPath('/v1/webhooks'), false);
});

const ERROR_ROUTES = [
  {
    name: 'unknown field',
    run: () => call('POST', '/v1/deductions', { ...ON_WEEKLY, typo_gross: '1.00' }),
  },
  {
    name: 'date out of range',
    run: () => call('POST', '/v1/deductions', { ...ON_WEEKLY, as_of: '2025-12-31' }),
  },
  {
    name: 'quebec refused',
    run: () => call('POST', '/v1/deductions', { ...ON_WEEKLY, province: 'QC' }),
  },
  {
    name: 'missing bearer',
    run: () => call('POST', '/v1/deductions', ON_WEEKLY, { authorization: null }),
  },
  {
    name: 'method not allowed',
    run: () => call('GET', '/v1/deductions'),
  },
  {
    name: 'unknown path',
    run: () => call('GET', '/v1/no-such-route'),
  },
  {
    name: 'unknown rule set',
    run: () => call('GET', '/v1/rules/2099-01-01'),
  },
  {
    name: 'malformed json',
    run: () =>
      call('POST', '/v1/deductions', '{', {
        'content-type': 'application/json',
      }),
  },
  {
    name: 'batch too large',
    run: () =>
      call('POST', '/v1/deductions/batch', {
        requests: Array.from({ length: 1001 }, () => ON_WEEKLY),
      }),
  },
  {
    name: 'unsafe webhook url',
    run: async () => {
      const world = createWorld();
      return call(
        'POST',
        '/v1/webhooks',
        { url: 'http://hooks.example.test/x' },
        { authorization: `Bearer ${world.testKey}` },
        world,
      );
    },
  },
  {
    name: 'unknown webhook id',
    run: async () => {
      const world = createWorld();
      return call(
        'GET',
        '/v1/webhooks/wh_ffffffffffffffffffffffffffffffff',
        undefined,
        { authorization: `Bearer ${world.testKey}` },
        world,
      );
    },
  },
  {
    name: 'invalid signup email',
    run: () => call('POST', '/v1/signup', { email: 'not-an-email' }),
  },
  {
    name: 'billing deferred',
    run: async () => {
      const world = createWorld();
      return call(
        'POST',
        '/v1/billing/checkout',
        { plan: 'enterprise' },
        { authorization: `Bearer ${world.liveKey}` },
        world,
      );
    },
  },
  {
    name: 'rate limited',
    run: async () => {
      const env = {
        ...createWorld().env,
        RATE_LIMIT_MAX: '1',
        RATE_LIMIT_WINDOW_MS: '60000',
      };
      const ip = { 'cf-connecting-ip': '198.51.100.9' };
      await handle(request('GET', '/v1/rules', undefined, ip), env, nodeEngine);
      const response = await handle(
        request('GET', '/v1/rules', undefined, ip),
        env,
        nodeEngine,
      );
      const text = await response.text();
      return { status: response.status, json: JSON.parse(text), text };
    },
  },
];

test('50. every error route is {code, message, docs} with no internals', async () => {
  assert.ok(ERROR_ROUTES.length >= 10);
  for (const route of ERROR_ROUTES) {
    const { status, json, text } = await route.run();
    assert.ok(status >= 400, `${route.name} status ${status}`);
    assertCleanError(json, route.name);
    assert.equal(LEAK.test(text), false, route.name);
  }
});

test('50. thrown store errors do not leak paths, SQL, or workerd', async () => {
  const world = createWorld();
  world.store.getAccount = () => {
    throw new Error(
      'D1_ERROR SQL: SELECT * FROM accounts at /usr/bin/workerd panicked in wrangler',
    );
  };
  const { status, json, text } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 500);
  assertCleanError(json, 'store throw');
  assert.equal(text.includes('SELECT'), false);
  assert.equal(text.includes('workerd'), false);
  assert.equal(text.includes('wrangler'), false);
  assert.equal(text.includes('/usr/bin'), false);
});

test('50. /health is liveness only', async () => {
  const { status, json } = await call('GET', '/health');
  assert.equal(status, 200);
  assert.deepEqual(Object.keys(json).sort(), ['status']);
  assert.equal(json.status, 'ok');
  assert.equal(json.engine_version, undefined);
  assert.equal(json.engine_build_sha256, undefined);
  assert.equal(json.rule_set_version, undefined);
  assert.equal(json.environment, undefined);
  assert.equal(json.d1, undefined);
  assert.equal(json.dependencies, undefined);
});

test('50. Worker has no unused bindings, no preview D1 at production, no body capture', () => {
  assert.doesNotMatch(WRANGLER, /preview_database_id/);
  assert.doesNotMatch(
    WRANGLER,
    /capture_request_body|logpush|tail_consumers|request_body/i,
  );
  const bindings = [
    ...new Set(
      [...WRANGLER.matchAll(/binding\s*=\s*"([^"]+)"/g)].map((row) => row[1]),
    ),
  ];
  assert.deepEqual([...bindings].sort(), ['ASSETS', 'DB']);
  assert.match(WRANGLER, /\[env\.staging\]/);
  assert.match(WRANGLER, /database_name\s*=\s*"takehome-staging"/);
  assert.doesNotMatch(
    WRANGLER,
    /\[env\.staging\][\s\S]*ECHO_VERIFY_URL/,
  );
  assert.match(WRANGLER, /\[observability\]/);
  assert.match(WRANGLER, /invocation_logs\s*=\s*true/);
  assert.doesNotMatch(WRANGLER, /persist\s*=\s*true/);
});
