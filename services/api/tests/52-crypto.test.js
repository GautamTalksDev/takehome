/**
 * A04:2025 Cryptographic Failures — items 27–31.
 */
import assert from 'node:assert/strict';
import { createHmac } from 'node:crypto';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, createWorld } from './helpers.js';
import {
  KEY_RANDOM_BYTES,
  WEBHOOK_SECRET_BYTES,
  equalDigest,
  hashKey,
  mintKey,
  mintWebhookSecret,
  randomHex,
} from '../src/keys.js';
import {
  TOLERANCE_SEC,
  signPayload,
  verifySignature,
  webhookPayload,
} from '../src/webhooks.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, '../../..');
const KEYS_SRC = readFileSync(path.join(HERE, '../src/keys.js'), 'utf8');
const HANDLER_SRC = readFileSync(path.join(HERE, '../src/handler.js'), 'utf8');
const WEBHOOKS_SRC = readFileSync(path.join(HERE, '../src/webhooks.js'), 'utf8');
const DOCS_SRC = readFileSync(
  path.join(REPO, 'web/site/src/pages/docs.astro'),
  'utf8',
);
const ADR = readFileSync(
  path.join(REPO, 'docs/ADR-004-api-key-hash.md'),
  'utf8',
);

const HEX = /^[0-9a-f]+$/;
const AMBIGUOUS = /[A-Z+/=OIl]/;

test('27. API keys are >=128 bits from getRandomValues in an unambiguous alphabet', () => {
  assert.match(KEYS_SRC, /crypto\.getRandomValues/);
  assert.doesNotMatch(KEYS_SRC.replace(/\/\*[\s\S]*?\*\//g, ''), /Math\.random/);
  assert.doesNotMatch(KEYS_SRC, /Date\.now\s*\(/);
  assert.ok(KEY_RANDOM_BYTES >= 16, '128 bits is 16 bytes');

  const sample = randomHex(KEY_RANDOM_BYTES);
  assert.equal(sample.length, KEY_RANDOM_BYTES * 2);
  assert.match(sample, HEX);
  assert.doesNotMatch(sample, AMBIGUOUS);
  assert.ok(sample.length * 4 >= 128);

  for (const kind of ['test', 'live']) {
    const key = mintKey(kind);
    const prefix = `np_${kind}_`;
    assert.ok(key.startsWith(prefix), key);
    const body = key.slice(prefix.length);
    assert.match(body, HEX);
    assert.doesNotMatch(body, AMBIGUOUS);
    assert.ok(body.length * 4 >= 128, `${kind} body bits ${body.length * 4}`);
  }

  const minted = new Set(Array.from({ length: 32 }, () => mintKey('live')));
  assert.equal(minted.size, 32);
});

test('28. key storage is SHA-256 of a high-entropy token, not a password hash', () => {
  assert.match(KEYS_SRC, /createHash\('sha256'\)|createHash\("sha256"\)/);
  assert.doesNotMatch(KEYS_SRC, /bcrypt|scrypt|argon2|pbkdf2/i);
  const key = mintKey('test');
  const digest = hashKey(key);
  assert.match(digest, /^[0-9a-f]{64}$/);
  assert.notEqual(digest, key);

  assert.match(ADR, /SHA-256|sha256/i);
  assert.match(ADR, /bcrypt/i);
  assert.match(ADR, /high-entropy|high entropy/i);
  assert.match(ADR, /lookup/i);
  assert.match(ADR, /password/i);
});

test('29. key hash and webhook HMAC compare with timingSafeEqual', () => {
  assert.match(KEYS_SRC, /timingSafeEqual/);
  assert.match(KEYS_SRC, /export function equalDigest/);
  assert.match(HANDLER_SRC, /equalDigest/);
  assert.match(WEBHOOKS_SRC, /equalDigest/);
  assert.doesNotMatch(
    WEBHOOKS_SRC,
    /expectedMac\s*===|gotMac\s*===|===\s*parsed\.v1/,
  );

  assert.equal(equalDigest('abcd', 'abcd'), true);
  assert.equal(equalDigest('abcd', 'abce'), false);
  assert.equal(equalDigest('abcd', 'abc'), false);
  assert.equal(equalDigest(Buffer.from('aa', 'hex'), Buffer.from('aa', 'hex')), true);
  assert.equal(equalDigest(Buffer.from('aa', 'hex'), Buffer.from('ab', 'hex')), false);

  const presented = hashKey('np_test_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
  const other = hashKey('np_test_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb');
  assert.equal(equalDigest(presented, presented), true);
  assert.equal(equalDigest(presented, other), false);
});

test('30. whsec_ is 256-bit, shown once, never retrievable', async () => {
  assert.equal(WEBHOOK_SECRET_BYTES, 32);
  const minted = mintWebhookSecret();
  assert.match(minted, /^whsec_[0-9a-f]{64}$/);
  assert.equal(minted.slice('whsec_'.length).length * 4, 256);

  const world = createWorld();
  const created = await call(
    'POST',
    '/v1/webhooks',
    { url: 'https://hooks.example.test/rules' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(created.status, 200, created.text);
  const secret = created.json.secret;
  assert.match(secret, /^whsec_[0-9a-f]{64}$/);
  assert.equal(secret.slice('whsec_'.length).length * 4, 256);

  const got = await call(
    'GET',
    `/v1/webhooks/${created.json.id}`,
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(got.status, 200, got.text);
  assert.equal(got.json.secret, undefined);
  assert.equal(JSON.stringify(got.json).includes(secret), false);

  const listed = await call(
    'GET',
    '/v1/webhooks',
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(listed.status, 200);
  assert.equal(JSON.stringify(listed.json).includes(secret), false);
  for (const row of listed.json.webhooks) {
    assert.equal(row.secret, undefined);
  }
});

test('31. webhook signatures bind timestamp and body; receivers reject outside the replay window', () => {
  assert.equal(TOLERANCE_SEC, 300);
  assert.match(WEBHOOKS_SRC, /\$\{t\}\.\$\{body\}|`\$\{t\}\.\$\{body\}`/);
  assert.match(DOCS_SRC, /takehome-signature/);
  assert.match(DOCS_SRC, /5 minutes/);
  assert.match(DOCS_SRC, /reject/i);
  assert.match(DOCS_SRC, /timestamp and body/i);

  const secret = 'whsec_test_secret';
  const body = webhookPayload({
    from: '2026-01-01',
    to: '2026-07-01',
    changed: ['BC'],
    occurredAt: '2026-09-16T00:00:00Z',
  });
  const t = 1_758_000_000;
  const header = signPayload(secret, body, t);
  assert.match(header, /^t=\d+,v1=[0-9a-f]{64}$/);

  const signedTogether = createHmac('sha256', secret)
    .update(`${t}.${body}`)
    .digest('hex');
  const signedBodyOnly = createHmac('sha256', secret).update(body).digest('hex');
  const v1 = header.split('v1=')[1];
  assert.equal(v1, signedTogether);
  assert.notEqual(v1, signedBodyOnly);

  assert.equal(verifySignature(secret, body, header, t), true);
  assert.equal(verifySignature(secret, body, header, t + TOLERANCE_SEC), true);
  assert.equal(verifySignature(secret, body, header, t - TOLERANCE_SEC), true);
  assert.equal(verifySignature(secret, body, header, t + TOLERANCE_SEC + 1), false);
  assert.equal(verifySignature(secret, body, header, t - TOLERANCE_SEC - 1), false);

  const bodyOnlyHeader = `t=${t},v1=${signedBodyOnly}`;
  assert.equal(verifySignature(secret, body, bodyOnlyHeader, t), false);
});
