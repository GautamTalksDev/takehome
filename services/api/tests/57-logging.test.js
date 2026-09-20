import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import {
  AUTH_FAIL_ALERT_AFTER,
  UnsafeLogError,
  log,
  resetSecurityLog,
} from '../src/log.js';
import { call, createWorld, ON_WEEKLY, request } from './helpers.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(here, '../../..');
const SRC = path.join(here, '../src');
const WRANGLER = readFileSync(path.join(here, '../wrangler.toml'), 'utf8');
const CALCULATOR = readFileSync(
  path.join(ROOT, 'web/site/src/components/Calculator.astro'),
  'utf8',
);
const DOCS = readFileSync(
  path.join(ROOT, 'web/site/src/pages/docs.astro'),
  'utf8',
);

const UNIQUE_GROSS = '8123.45';
const DEAD_LIVE_KEY = 'np_live_ffffffffffffffffffffffffffffffff';

async function captureLog(fn) {
  const lines = [];
  const original = console.log;
  console.log = (...args) => {
    lines.push(args.map(String).join(' '));
  };
  try {
    await fn();
  } finally {
    console.log = original;
  }
  return lines;
}

test('48. logging helper rejects an object containing a gross_pay field', () => {
  let called = false;
  const original = console.log;
  console.log = () => {
    called = true;
  };
  try {
    assert.throws(
      () => log({ path: '/v1/deductions', gross_pay: '1000.00' }),
      UnsafeLogError,
    );
    assert.throws(
      () => log({ nested: { gross_pay: '1000.00' } }),
      UnsafeLogError,
    );
    assert.equal(called, false);
    log({ type: 'quota_402', path: '/v1/deductions' });
    assert.equal(called, true);
  } finally {
    console.log = original;
  }
});

test('48. Worker never logs a request body and observability excludes bodies', async () => {
  assert.match(CALCULATOR, /No salary figure leaves the browser/);
  assert.match(DOCS, /does not store calculation inputs/i);
  assert.match(WRANGLER, /\[observability\]/);
  assert.match(WRANGLER, /invocation_logs\s*=\s*true/);
  assert.doesNotMatch(WRANGLER, /persist\s*=\s*true/);
  assert.doesNotMatch(
    WRANGLER,
    /capture_request_body|logpush|tail_consumers|request_body/i,
  );

  for (const name of readdirSync(SRC).filter((file) => file.endsWith('.js'))) {
    const text = readFileSync(path.join(SRC, name), 'utf8');
    const stripped = text
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/\/\/.*$/gm, '');
    assert.doesNotMatch(stripped, /log\(\s*(raw|payload|body)\s*\)/, name);
    if (name === 'log.js') {
      continue;
    }
    assert.doesNotMatch(
      stripped,
      /console\.(log|info|warn|error|debug)\s*\(/,
      name,
    );
  }

  const world = createWorld();
  const lines = await captureLog(async () => {
    const { status, json } = await call(
      'POST',
      '/v1/deductions',
      { ...ON_WEEKLY, gross_pay: UNIQUE_GROSS },
      { authorization: `Bearer ${world.testKey}` },
      world,
    );
    assert.equal(status, 200, json?.error?.message);
  });
  const blob = lines.join('\n');
  assert.equal(blob.includes(UNIQUE_GROSS), false);
  assert.equal(blob.includes('gross_pay'), false);
});

test('49. repeated auth failures from one IP are logged and alerted', async () => {
  resetSecurityLog();
  const world = createWorld();
  for (let i = 0; i < AUTH_FAIL_ALERT_AFTER - 1; i += 1) {
    const { status } = await call(
      'POST',
      '/v1/deductions',
      ON_WEEKLY,
      {
        authorization: `Bearer ${DEAD_LIVE_KEY}`,
        'cf-connecting-ip': '198.51.100.20',
      },
      world,
    );
    assert.equal(status, 401);
  }
  assert.equal(
    world.alerts.filter((row) => row.type === 'auth_failures').length,
    0,
  );

  const fifth = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    {
      authorization: `Bearer ${DEAD_LIVE_KEY}`,
      'cf-connecting-ip': '198.51.100.20',
    },
    world,
  );
  assert.equal(fifth.status, 401);
  const bursts = world.alerts.filter((row) => row.type === 'auth_failures');
  assert.equal(bursts.length, 1);
  assert.equal(bursts[0].ip, '198.51.100.20');
  assert.equal(JSON.stringify(bursts).includes('gross_pay'), false);

  const other = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    {
      authorization: `Bearer ${DEAD_LIVE_KEY}`,
      'cf-connecting-ip': '198.51.100.21',
    },
    world,
  );
  assert.equal(other.status, 401);
  assert.equal(
    world.alerts.filter((row) => row.type === 'auth_failures').length,
    1,
  );
});

test('49. quota 402s, webhook failures, 5xx, and untyped engine errors alert; typed engine errors do not', async () => {
  const quotaWorld = createWorld('developer');
  quotaWorld.store.setUsage(quotaWorld.account.id, '2026-09-01', 100_000);
  const quota = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${quotaWorld.liveKey}` },
    quotaWorld,
  );
  assert.equal(quota.status, 402);
  assert.equal(
    quotaWorld.alerts.filter((row) => row.type === 'quota_402').length,
    1,
  );
  assert.equal(JSON.stringify(quotaWorld.alerts).includes('gross_pay'), false);

  const hookWorld = createWorld();
  hookWorld.env.WEBHOOK_FETCH = async () => ({ ok: false, status: 500 });
  hookWorld.env.WEBHOOK_SLEEP = async () => {};
  const created = await call(
    'POST',
    '/v1/webhooks',
    { url: 'https://hooks.example.test/rules' },
    { authorization: `Bearer ${hookWorld.testKey}` },
    hookWorld,
  );
  assert.equal(created.status, 200, created.text);
  const dispatch = await call(
    'POST',
    '/v1/webhooks/dispatch',
    { from: '2026-01-01', to: '2026-07-01' },
    { authorization: `Bearer ${hookWorld.testKey}` },
    hookWorld,
  );
  assert.equal(dispatch.status, 200, dispatch.text);
  assert.equal(dispatch.json.deliveries[0].status, 'failed');
  assert.ok(
    hookWorld.alerts.some((row) => row.type === 'webhook_delivery_failed'),
  );
  assert.equal(
    JSON.stringify(hookWorld.alerts).includes(created.json.secret),
    false,
  );

  const fiveWorld = createWorld();
  fiveWorld.store.getAccount = () => {
    throw new Error('store unavailable');
  };
  const boom = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${fiveWorld.testKey}` },
    fiveWorld,
  );
  assert.equal(boom.status, 500);
  assert.ok(fiveWorld.alerts.some((row) => row.type === 'http_5xx'));

  const typedWorld = createWorld();
  const typed = await call(
    'POST',
    '/v1/deductions',
    { ...ON_WEEKLY, typo_gross: '1.00' },
    { authorization: `Bearer ${typedWorld.testKey}` },
    typedWorld,
  );
  assert.equal(typed.status, 400);
  assert.equal(typed.json.error.code, 'unknown_field');
  assert.equal(
    typedWorld.alerts.filter((row) => row.type === 'http_5xx').length,
    0,
  );
  assert.equal(
    typedWorld.alerts.filter((row) => row.type === 'engine_error').length,
    0,
  );

  const engineWorld = createWorld();
  const broken = {
    ...nodeEngine,
    calculate(input) {
      const payload = JSON.parse(String(input));
      if (payload.gross_pay === UNIQUE_GROSS) {
        return JSON.stringify({
          error: { code: 'engine', message: 'internal engine failure' },
        });
      }
      return nodeEngine.calculate(input);
    },
  };
  const engineResponse = await handle(
    request(
      'POST',
      '/v1/deductions',
      { ...ON_WEEKLY, gross_pay: UNIQUE_GROSS },
      { authorization: `Bearer ${engineWorld.testKey}` },
    ),
    engineWorld.env,
    broken,
  );
  assert.equal(engineResponse.status, 500);
  assert.ok(
    engineWorld.alerts.some(
      (row) => row.type === 'http_5xx' || row.type === 'engine_error',
    ),
  );
  assert.equal(JSON.stringify(engineWorld.alerts).includes(UNIQUE_GROSS), false);
});

test('50. a quota 402 reaches ALERTS and ALERT_EMAIL end to end', async () => {
  const world = createWorld('developer');
  world.env.ALERT_EMAIL = 'ops@takehome.example';
  world.store.setUsage(world.account.id, '2026-09-01', 100_000);
  const { status } = await call(
    'POST',
    '/v1/deductions',
    { ...ON_WEEKLY, gross_pay: UNIQUE_GROSS },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 402);
  const alerts = world.alerts.filter((row) => row.type === 'quota_402');
  assert.equal(alerts.length, 1);
  const mail = world.mailbox.filter(
    (row) => row.to === 'ops@takehome.example',
  );
  assert.equal(mail.length, 1);
  assert.match(mail[0].subject, /quota_402/);
  const blob = JSON.stringify({ alerts, mail });
  assert.equal(blob.includes('gross_pay'), false);
  assert.equal(blob.includes(UNIQUE_GROSS), false);
});
