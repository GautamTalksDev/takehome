/**
 * A07:2025 Authentication Failures — items 40–44.
 */
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY } from './helpers.js';
import {
  AUTH_DUMMY_SECRET,
  KEY_RANDOM_BYTES,
  equalDigest,
  hashKey,
  keyKind,
  mintKey,
  redactSecrets,
} from '../src/keys.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.join(HERE, '../src');
const HANDLER = readFileSync(path.join(SRC, 'handler.js'), 'utf8');
const KEYS = readFileSync(path.join(SRC, 'keys.js'), 'utf8');
const WRANGLER = readFileSync(path.join(HERE, '../wrangler.toml'), 'utf8');

const HEX_LEN = KEY_RANDOM_BYTES * 2;
const KEY_BODY = new RegExp(`np_(?:test|live)_[0-9a-f]{${HEX_LEN}}`);

function authenticateSource() {
  const match = HANDLER.match(
    /async function authenticate\(request, env, engine\) \{[\s\S]*?\nasync function /,
  );
  assert.ok(match, 'authenticate() must exist');
  return match[0];
}

function authFailure(res) {
  return {
    status: res.status,
    code: res.json.error.code,
    message: res.json.error.message,
    docs: res.json.error.docs,
    www: res.response.headers.get('www-authenticate'),
  };
}

async function deductionsWith(world, secret) {
  return call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${secret}` },
    world,
  );
}

test('40. keys cannot be recovered; a lost key is regenerated once', async () => {
  const world = createWorld();
  const listed = await call(
    'GET',
    '/v1/keys',
    undefined,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(listed.status, 200, listed.text);
  assert.equal(listed.json.keys.length, 2);
  const kinds = listed.json.keys.map((row) => row.kind).sort();
  assert.deepEqual(kinds, ['live', 'test']);
  for (const row of listed.json.keys) {
    assert.equal(row.prefix, `np_${row.kind}_`);
    assert.equal(row.key, undefined);
    assert.equal(row.secret, undefined);
    assert.equal(row.hash, undefined);
  }
  assert.doesNotMatch(listed.text, KEY_BODY);
  assert.equal(listed.text.includes(world.testKey), false);
  assert.equal(listed.text.includes(world.liveKey), false);

  const missing = await call(
    'GET',
    `/v1/keys/${encodeURIComponent(world.liveKey)}`,
    undefined,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(missing.status, 404);
  assert.doesNotMatch(missing.text, KEY_BODY);

  const rotated = await call(
    'POST',
    '/v1/keys/rotate',
    { kind: 'live' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(rotated.status, 200, rotated.text);
  assert.equal(rotated.json.kind, 'live');
  assert.match(rotated.json.key, new RegExp(`^np_live_[0-9a-f]{${HEX_LEN}}$`));
  assert.notEqual(rotated.json.key, world.liveKey);
  assert.match(rotated.json.message, /once|previous/i);

  const oldLive = await deductionsWith(world, world.liveKey);
  assert.equal(oldLive.status, 401);
  assert.equal(oldLive.json.error.code, 'unauthorized');

  const newLive = await deductionsWith(world, rotated.json.key);
  assert.equal(newLive.status, 200, newLive.text);

  const listedAfter = await call(
    'GET',
    '/v1/keys',
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(listedAfter.status, 200);
  assert.doesNotMatch(listedAfter.text, new RegExp(rotated.json.key));
  const liveRows = listedAfter.json.keys.filter((row) => row.kind === 'live');
  assert.equal(liveRows.length, 1);

  assert.match(HANDLER, /test_key|live_key/);
  const issuance = HANDLER.match(
    /async function verifySignup[\s\S]*?\nasync function /,
  )[0];
  assert.match(issuance, /test_key:/);
  assert.match(issuance, /live_key:/);
  const rotateFn = HANDLER.match(
    /async function rotateAccountKey[\s\S]*?\nasync function /,
  )[0];
  assert.match(rotateFn, /mintKey/);
  assert.doesNotMatch(rotateFn, /getKeyByHash\([^)]*\)[\s\S]{0,80}return json\(200/);
});

test('41. unknown-format and unknown valid-format keys share a path and the same error', async () => {
  const authFn = authenticateSource();
  assert.match(authFn, /hashKey\(/);
  assert.match(authFn, /getKeyByHash/);
  assert.match(authFn, /equalDigest/);
  assert.match(authFn, /AUTH_DUMMY_SECRET/);
  assert.doesNotMatch(authFn, /if\s*\(\s*!secret\s*\)/);
  assert.doesNotMatch(authFn, /if\s*\(\s*!kind\s*\)/);
  const hashAt = authFn.indexOf('hashKey');
  const lookupAt = authFn.indexOf('getKeyByHash');
  const digestAt = authFn.indexOf('equalDigest');
  const denyAt = authFn.indexOf('unauthorized');
  assert.ok(hashAt < lookupAt && lookupAt < digestAt && digestAt < denyAt);

  assert.equal(keyKind('not-a-key'), null);
  assert.equal(keyKind(`np_live_${'f'.repeat(HEX_LEN - 1)}`), null);
  const unknownValid = `np_live_${'f'.repeat(HEX_LEN)}`;
  assert.equal(keyKind(unknownValid), 'live');
  assert.match(AUTH_DUMMY_SECRET, new RegExp(`^np_live_[0-9a-f]{${HEX_LEN}}$`));
  assert.equal(keyKind(AUTH_DUMMY_SECRET), 'live');

  const world = createWorld();
  const unknown = await deductionsWith(world, unknownValid);
  const invalid = await deductionsWith(world, 'totally-not-a-key');
  const short = await deductionsWith(world, 'np_live_abcd');
  assert.deepEqual(authFailure(unknown), authFailure(invalid));
  assert.deepEqual(authFailure(unknown), authFailure(short));
  assert.equal(unknown.status, 401);
  assert.equal(unknown.json.error.code, 'unauthorized');
  assert.equal(unknown.response.headers.get('www-authenticate'), 'Bearer');

  const presented = hashKey(unknownValid);
  const stored = world.store.getKeyByHash(presented);
  assert.equal(stored, null);
  assert.equal(equalDigest(presented, hashKey(AUTH_DUMMY_SECRET)), false);
});

test('42. np_test_ cannot make live calls and does not appear in live usage', async () => {
  const world = createWorld();
  const testCalc = await deductionsWith(world, world.testKey);
  assert.equal(testCalc.status, 200);
  assert.equal(testCalc.response.headers.get('x-usage-limit'), 'unlimited');
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 0);

  const liveCalc = await deductionsWith(world, world.liveKey);
  assert.equal(liveCalc.status, 200);
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 1);
  assert.equal(liveCalc.response.headers.get('x-usage-limit'), '100000');
  assert.equal(liveCalc.response.headers.get('x-usage-remaining'), '99999');

  const testAsLive = `np_live_${world.testKey.slice('np_test_'.length)}`;
  const liveAsTest = `np_test_${world.liveKey.slice('np_live_'.length)}`;
  assert.equal(keyKind(testAsLive), 'live');
  assert.equal(keyKind(liveAsTest), 'test');
  const swappedLive = await deductionsWith(world, testAsLive);
  const swappedTest = await deductionsWith(world, liveAsTest);
  assert.equal(swappedLive.status, 401);
  assert.equal(swappedTest.status, 401);
  assert.deepEqual(authFailure(swappedLive), authFailure(swappedTest));
  assert.equal(world.store.getUsage(world.account.id, '2026-09-01'), 1);

  const testCheckout = await call(
    'POST',
    '/v1/billing/checkout',
    { plan: 'starter' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(testCheckout.status, 401);
  assert.equal(testCheckout.json.error.code, 'unauthorized');
  assert.equal(world.stripeSessions.length, 0);

  const liveCheckout = await call(
    'POST',
    '/v1/billing/checkout',
    { plan: 'starter' },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(liveCheckout.status, 501, liveCheckout.text);
  assert.equal(liveCheckout.json.error.code, 'billing_unavailable');
  assert.equal(world.stripeSessions.length, 0);
});

test('43. a revoked key fails on the next request, not at cache expiry', async () => {
  const authFn = authenticateSource();
  assert.doesNotMatch(authFn, /cache|ttl|expires/i);
  assert.match(authFn, /revoked/);
  assert.match(authFn, /getKeyByHash/);

  const world = createWorld();
  const before = await deductionsWith(world, world.liveKey);
  assert.equal(before.status, 200);

  const revoked = await call(
    'POST',
    '/v1/keys/revoke',
    { kind: 'live' },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(revoked.status, 200, revoked.text);
  assert.equal(revoked.json.kind, 'live');
  assert.equal(revoked.json.revoked, true);

  const row = world.store.getKeyByHash(hashKey(world.liveKey));
  assert.equal(Boolean(row?.revoked), true);

  const first = await deductionsWith(world, world.liveKey);
  const second = await deductionsWith(world, world.liveKey);
  assert.equal(first.status, 401);
  assert.equal(second.status, 401);
  assert.equal(first.json.error.code, 'unauthorized');
  assert.deepEqual(authFailure(first), authFailure(second));

  const testStill = await deductionsWith(world, world.testKey);
  assert.equal(testStill.status, 200);

  const listed = await call(
    'GET',
    '/v1/keys',
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(listed.json.keys.some((row) => row.kind === 'live'), false);
});

test('44. never log a key, even truncated, even in an error', async () => {
  const methods = ['log', 'info', 'warn', 'error', 'debug'];
  const original = Object.fromEntries(
    methods.map((name) => [name, console[name].bind(console)]),
  );
  const lines = [];
  for (const name of methods) {
    console[name] = (...args) => {
      lines.push(
        args
          .map((arg) => (typeof arg === 'string' ? arg : JSON.stringify(arg)))
          .join(' '),
      );
    };
  }

  const world = createWorld();
  const unknown = `np_live_${'c'.repeat(HEX_LEN)}`;
  let rotateKey = '';
  try {
    const ok = await deductionsWith(world, world.testKey);
    assert.equal(ok.status, 200);
    const bad = await deductionsWith(world, unknown);
    assert.equal(bad.status, 401);
    assert.doesNotMatch(bad.text, new RegExp(unknown));
    assert.doesNotMatch(bad.text, new RegExp(world.testKey));
    assert.doesNotMatch(bad.text, /np_(?:test|live)_[0-9a-f]{8}/);

    const rotated = await call(
      'POST',
      '/v1/keys/rotate',
      { kind: 'test' },
      { authorization: `Bearer ${world.liveKey}` },
      world,
    );
    assert.equal(rotated.status, 200);
    rotateKey = rotated.json.key;

    const blob = lines.join('\n');
    for (const secret of [world.testKey, world.liveKey, unknown, rotateKey]) {
      assert.equal(blob.includes(secret), false, `logged full key ${secret}`);
      assert.equal(
        blob.includes(secret.slice(0, 16)),
        false,
        `logged truncated key ${secret.slice(0, 16)}`,
      );
    }
  } finally {
    for (const name of methods) {
      console[name] = original[name];
    }
  }

  const sample = mintKey('live');
  assert.equal(redactSecrets(`Authorization: Bearer ${sample}`), 'Authorization: Bearer [redacted]');
  assert.equal(
    redactSecrets(sample.slice(0, 16)),
    '[redacted]',
  );
  assert.doesNotMatch(redactSecrets(`err ${sample} ${sample.slice(0, 12)}`), /np_live_/);

  for (const name of readdirSync(SRC).filter((file) => file.endsWith('.js'))) {
    const text = readFileSync(path.join(SRC, name), 'utf8');
    const stripped = text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/.*$/gm, '');
    if (name === 'log.js') {
      assert.match(stripped, /redactSecrets/);
      continue;
    }
    assert.doesNotMatch(
      stripped,
      /console\.(log|info|warn|error|debug)\s*\(/,
      name,
    );
  }

  assert.doesNotMatch(
    WRANGLER,
    /capture_request_body|capture_request_headers|head_sampling|request_headers/i,
  );
  assert.match(KEYS, /export function redactSecrets/);
});
