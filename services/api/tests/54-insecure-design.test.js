/**
 * A06:2025 Insecure Design — items 36–39.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY, request, BASE_ENV, newStore } from './helpers.js';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import {
  TOKEN_RANDOM_BYTES,
  equalDigest,
  hashKey,
  randomHex,
} from '../src/keys.js';
import { resetRateLimits } from '../src/rate-limit.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WRANGLER = readFileSync(path.join(HERE, '../wrangler.toml'), 'utf8');
const HANDLER = readFileSync(path.join(HERE, '../src/handler.js'), 'utf8');
const KEYS = readFileSync(path.join(HERE, '../src/keys.js'), 'utf8');

const SALARY = '91827.43';
const SALARY_REQUEST = { ...ON_WEEKLY, gross_pay: SALARY };

test('36. unauthenticated endpoints are IP rate limited', async () => {
  resetRateLimits();
  const env = { ...BASE_ENV, RATE_LIMIT_MAX: '2', RATE_LIMIT_WINDOW_MS: '60000' };
  const ip = { 'cf-connecting-ip': '198.51.100.36' };
  assert.equal(
    (await handle(request('GET', '/v1/rules', undefined, ip), env, nodeEngine))
      .status,
    200,
  );
  assert.equal(
    (await handle(request('GET', '/v1/rules', undefined, ip), env, nodeEngine))
      .status,
    200,
  );
  const third = await handle(
    request('GET', '/v1/rules', undefined, ip),
    env,
    nodeEngine,
  );
  assert.equal(third.status, 429);
  const json = await third.json();
  assert.equal(json.error.code, 'rate_limited');
});

test('36. signup and verify are limited separately and harder than public listings', async () => {
  resetRateLimits();
  const world = createWorld();
  world.env.RATE_LIMIT_MAX = '50';
  world.env.SIGNUP_RATE_LIMIT_MAX = '2';
  world.env.VERIFY_RATE_LIMIT_MAX = '2';
  world.env.RATE_LIMIT_WINDOW_MS = '60000';
  const ip = { 'cf-connecting-ip': '198.51.100.37' };

  for (let i = 0; i < 2; i += 1) {
    const signed = await handle(
      request(
        'POST',
        '/v1/signup',
        { email: `rate-${i}@example.com` },
        ip,
      ),
      world.env,
      nodeEngine,
    );
    assert.equal(signed.status, 200, await signed.text());
  }
  const blockedSignup = await handle(
    request('POST', '/v1/signup', { email: 'rate-2@example.com' }, ip),
    world.env,
    nodeEngine,
  );
  assert.equal(blockedSignup.status, 429);
  assert.equal((await blockedSignup.json()).error.code, 'rate_limited');

  const listing = await handle(
    request('GET', '/v1/rules', undefined, ip),
    world.env,
    nodeEngine,
  );
  assert.equal(listing.status, 200, 'signup must not spend the public bucket');

  const verifyIp = { 'cf-connecting-ip': '198.51.100.38' };
  for (let i = 0; i < 2; i += 1) {
    const res = await handle(
      request('GET', '/v1/signup/verify?token=deadbeef', undefined, verifyIp),
      world.env,
      nodeEngine,
    );
    assert.ok(res.status === 400 || res.status === 429);
  }
  const blockedVerify = await handle(
    request('GET', '/v1/signup/verify?token=deadbeef', undefined, verifyIp),
    world.env,
    nodeEngine,
  );
  assert.equal(blockedVerify.status, 429);

  world.env.RATE_LIMIT_MAX = '1';
  const world2 = createWorld();
  world2.env.RATE_LIMIT_MAX = '1';
  const mixed = { 'cf-connecting-ip': '198.51.100.39' };
  assert.equal(
    (
      await handle(
        request('GET', '/v1/rules', undefined, mixed),
        world2.env,
        nodeEngine,
      )
    ).status,
    200,
  );
  assert.equal(
    (
      await handle(
        request('GET', '/v1/rules', undefined, mixed),
        world2.env,
        nodeEngine,
      )
    ).status,
    429,
  );
  const calc = await handle(
    request('POST', '/v1/deductions', ON_WEEKLY, {
      ...mixed,
      authorization: `Bearer ${world2.testKey}`,
    }),
    world2.env,
    nodeEngine,
  );
  assert.equal(calc.status, 200, 'keyed deductions are not the public IP bucket');
});

test('37. verification tokens are 128-bit, single-use, expiring, and compared constantly', async () => {
  assert.ok(TOKEN_RANDOM_BYTES >= 16);
  assert.match(KEYS, /crypto\.getRandomValues/);
  assert.match(HANDLER, /equalDigest/);
  const sample = randomHex(TOKEN_RANDOM_BYTES);
  assert.equal(sample.length * 4, TOKEN_RANDOM_BYTES * 8);
  assert.ok(sample.length * 4 >= 128);

  const presented = hashKey(sample);
  assert.equal(equalDigest(presented, presented), true);
  assert.equal(equalDigest(presented, hashKey(randomHex(TOKEN_RANDOM_BYTES))), false);

  const world = createWorld();
  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'once@example.com' },
    {},
    world,
  );
  assert.equal(signed.status, 200, signed.text);
  const token = world.mailbox[0].token;
  assert.match(token, /^[0-9a-f]{32,}$/);
  assert.ok(token.length * 4 >= 128);

  const first = await call(
    'GET',
    `/v1/signup/verify?token=${token}`,
    undefined,
    {},
    world,
  );
  assert.equal(first.status, 200, first.text);
  const reused = await call(
    'GET',
    `/v1/signup/verify?token=${token}`,
    undefined,
    {},
    world,
  );
  assert.equal(reused.status, 400);
  assert.equal(reused.json.error.code, 'invalid_request');

  const expiredStore = newStore();
  const mailbox = [];
  const expiredWorld = {
    env: { ...BASE_ENV, STORE: expiredStore, MAILBOX: mailbox },
  };
  const account = expiredStore.createAccount({
    email: 'expired@example.com',
    email_verified: false,
  });
  const stale = randomHex(TOKEN_RANDOM_BYTES);
  expiredStore.insertEmailToken({
    hash: hashKey(stale),
    account_id: account.id,
    expires_at: '2020-01-01T00:00:00Z',
  });
  const expired = await call(
    'GET',
    `/v1/signup/verify?token=${stale}`,
    undefined,
    {},
    expiredWorld,
  );
  assert.equal(expired.status, 400);
  assert.match(expired.json.error.message, /expired/i);
});

test('38. a calculation writes no salary figures to D1 or KV', async () => {
  assert.doesNotMatch(WRANGLER, /kv_namespaces/);
  assert.doesNotMatch(WRANGLER, /\[\[kv/i);
  const world = createWorld();
  assert.equal(world.env.KV, undefined);

  const before = JSON.stringify({
    dump: world.store.dump(),
    usage: [...world.store.usage.entries()],
  });
  const { status, json } = await call(
    'POST',
    '/v1/deductions',
    SALARY_REQUEST,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(status, 200, json?.error?.message);
  const afterTest = JSON.stringify({
    dump: world.store.dump(),
    usage: [...world.store.usage.entries()],
  });
  assert.equal(afterTest, before);
  assert.doesNotMatch(afterTest, new RegExp(SALARY.replace('.', '\\.')));
  assert.doesNotMatch(afterTest, /gross_pay/);

  const live = await call(
    'POST',
    '/v1/deductions',
    SALARY_REQUEST,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(live.status, 200);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 1);
  const afterLive = JSON.stringify({
    dump: world.store.dump(),
    usage: [...world.store.usage.entries()],
  });
  assert.doesNotMatch(afterLive, new RegExp(SALARY.replace('.', '\\.')));
  assert.doesNotMatch(afterLive, /gross_pay/);
  for (const value of world.store.usage.values()) {
    assert.equal(typeof value, 'number');
  }
});

test('39. extra keys on one account cannot multiply the 1,000 free-tier cap', async () => {
  const store = newStore();
  const mailbox = [];
  const world = { env: { ...BASE_ENV, STORE: store, MAILBOX: mailbox } };
  const email = 'one-account@example.com';
  assert.equal(
    (await call('POST', '/v1/signup', { email }, {}, world)).status,
    200,
  );
  assert.equal(
    (await call('POST', '/v1/signup', { email }, {}, world)).status,
    200,
  );
  assert.equal(mailbox.length, 2);
  const first = await call(
    'GET',
    `/v1/signup/verify?token=${mailbox[0].token}`,
    undefined,
    {},
    world,
  );
  assert.equal(first.status, 200, first.text);
  const liveKeysAfterFirst = [...store.keys.values()].filter(
    (row) => row.kind === 'live',
  );
  assert.equal(liveKeysAfterFirst.length, 1);

  const second = await call(
    'GET',
    `/v1/signup/verify?token=${mailbox[1].token}`,
    undefined,
    {},
    world,
  );
  assert.equal(second.status, 400);
  assert.equal(
    [...store.keys.values()].filter((row) => row.kind === 'live').length,
    1,
  );

  const liveKey = first.json.live_key;
  store.setUsage([...store.accounts.values()][0].id, '2026-09-01', 100_000);
  const over = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${liveKey}` },
    world,
  );
  assert.equal(over.status, 402);
  assert.equal(over.json.error.code, 'plan_limit');
});
