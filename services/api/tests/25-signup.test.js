import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY, request, newStore } from './helpers.js';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { BASE_ENV } from './helpers.js';
import { echoVerifyUrlEnabled } from '../src/handler.js';

const WRANGLER = readFileSync(
  path.join(path.dirname(fileURLToPath(import.meta.url)), '../wrangler.toml'),
  'utf8',
);

function assertSignupBodyHasNoEcho(serialized) {
  assert.doesNotMatch(serialized, /https?:\/\//i);
  assert.doesNotMatch(serialized, /\/signup\/verify/i);
  assert.doesNotMatch(serialized, /"[^"]*url[^"]*"\s*:/i);
  assert.doesNotMatch(serialized, /"[^"]*token[^"]*"\s*:/i);
  assert.doesNotMatch(serialized, /[0-9a-f]{32}/i);
}

test('25. signup is email, verify, keys, first call', async () => {
  const store = newStore();
  const mailbox = [];
  const env = { ...BASE_ENV, STORE: store, MAILBOX: mailbox };
  const world = { env, testKey: null };

  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'ada@example.com' },
    {},
    world,
  );
  assert.equal(signed.status, 200);
  assert.equal(signed.json.test_key, undefined);
  assert.equal(signed.json.live_key, undefined);
  assert.equal(signed.json.verify_url, undefined);
  assert.equal(mailbox.length, 1);
  assert.equal(mailbox[0].to, 'ada@example.com');
  assert.ok(mailbox[0].token);
  assert.doesNotMatch(JSON.stringify(store.dump()), new RegExp(mailbox[0].token));

  const verified = await call(
    'GET',
    `/v1/signup/verify?token=${mailbox[0].token}`,
    undefined,
    {},
    world,
  );
  assert.equal(verified.status, 200);
  assert.match(verified.json.test_key, /^np_test_/);
  assert.match(verified.json.live_key, /^np_live_/);

  const first = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${verified.json.test_key}` },
    world,
  );
  assert.equal(first.status, 200);
  assert.equal(first.json.employee.net_pay, '800.79');
});

test('25. signup copy has no sales call or onboarding', async () => {
  const store = newStore();
  const env = { ...BASE_ENV, STORE: store, MAILBOX: [] };
  const res = await handle(
    request('POST', '/v1/signup', { email: 'bob@example.com' }),
    env,
    nodeEngine,
  );
  const json = await res.json();
  assert.doesNotMatch(json.message, /sales/i);
  assert.doesNotMatch(json.message, /onboarding/i);
  assert.doesNotMatch(json.message, /demo/i);
});

test('25. ECHO_VERIFY_URL unset: signup body has no URL, token, or resembling field', async () => {
  assert.equal(BASE_ENV.ECHO_VERIFY_URL, undefined);
  assert.equal(echoVerifyUrlEnabled(BASE_ENV), false);
  const store = newStore();
  const mailbox = [];
  const env = { ...BASE_ENV, STORE: store, MAILBOX: mailbox };
  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'no-echo@example.com' },
    {},
    { env, testKey: null },
  );
  assert.equal(signed.status, 200);
  assertSignupBodyHasNoEcho(JSON.stringify(signed.json));
  assertSignupBodyHasNoEcho(signed.text);
  assert.ok(mailbox[0].token);
});

test('25. ECHO_VERIFY_URL default-deny for empty, 0, false, true, and junk', async () => {
  const denied = [undefined, '', '0', 'false', 'true', 'yes', 'TRUE', 'on', '2'];
  for (const value of denied) {
    assert.equal(echoVerifyUrlEnabled({ ECHO_VERIFY_URL: value }), false, String(value));
    const store = newStore();
    const env = { ...BASE_ENV, STORE: store, MAILBOX: [], ECHO_VERIFY_URL: value };
    const signed = await call(
      'POST',
      '/v1/signup',
      { email: `deny-${String(value)}@example.com` },
      {},
      { env, testKey: null },
    );
    assert.equal(signed.status, 200, String(value));
    assertSignupBodyHasNoEcho(JSON.stringify(signed.json));
  }
});

test('25. ECHO_VERIFY_URL=1 still echoes the verify URL for the local harness', async () => {
  assert.equal(echoVerifyUrlEnabled({ ECHO_VERIFY_URL: '1' }), true);
  const store = newStore();
  const mailbox = [];
  const env = { ...BASE_ENV, STORE: store, MAILBOX: mailbox, ECHO_VERIFY_URL: '1' };
  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'echo-on@example.com' },
    {},
    { env, testKey: null },
  );
  assert.equal(signed.status, 200);
  assert.match(signed.json.verify_url, /\/signup\/verify\/\?token=/);
  const token = new URL(signed.json.verify_url).searchParams.get('token');
  assert.equal(token, mailbox[0].token);
});

test('25. production wrangler.toml does not contain ECHO_VERIFY_URL', () => {
  assert.doesNotMatch(WRANGLER, /^\s*ECHO_VERIFY_URL\s*=/m);
  assert.equal(WRANGLER.includes('ECHO_VERIFY_URL'), false);
  for (const sibling of [
    'CLOCK_DATE',
    'MAILBOX',
    'STORE',
    'WEBHOOK_FETCH',
    'WEBHOOK_SLEEP',
    'WEBHOOK_RESOLVE',
    'ALERTS',
  ]) {
    assert.doesNotMatch(
      WRANGLER,
      new RegExp(`^\\s*${sibling}\\s*=`, 'm'),
      `${sibling} as a wrangler var would weaken a control`,
    );
  }
});
