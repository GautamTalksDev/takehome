import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld } from './helpers.js';
import {
  deliverWithRetry,
  signPayload,
  verifySignature,
  webhookPayload,
} from '../src/webhooks.js';

test('46. HMAC signature verification accepts the signed body and rejects a tamper', () => {
  const secret = 'whsec_test_secret';
  const body = webhookPayload({
    from: '2026-01-01',
    to: '2026-07-01',
    changed: ['BC', 'NL', 'PE'],
    occurredAt: '2026-09-16T00:00:00Z',
  });
  const header = signPayload(secret, body, 1_758_000_000);
  assert.equal(verifySignature(secret, body, header, 1_758_000_000), true);
  assert.equal(verifySignature('whsec_other', body, header, 1_758_000_000), false);
  assert.equal(
    verifySignature(secret, body.replace('BC', 'ON'), header, 1_758_000_000),
    false,
  );
  assert.equal(verifySignature(secret, body, header, 1_758_000_000 + 301), false);
});

test('46. deliver retries with backoff then succeeds', async () => {
  const sleeps = [];
  let hits = 0;
  const result = await deliverWithRetry({
    url: 'https://hooks.example.test/rules',
    body: '{"event":"rule_set.changed"}',
    secret: 'whsec_test',
    timestampSec: 1_758_000_000,
    fetchImpl: async () => {
      hits += 1;
      if (hits < 3) {
        return { ok: false, status: 503 };
      }
      return { ok: true, status: 200 };
    },
    sleep: async (ms) => {
      sleeps.push(ms);
    },
  });
  assert.equal(result.ok, true);
  assert.equal(result.attempts, 3);
  assert.deepEqual(sleeps, [1000, 2000]);
});

test('46. register, dispatch, delivery log, and replay', async () => {
  const world = createWorld();
  const fetches = [];
  let failNext = 2;
  world.env.WEBHOOK_FETCH = async (url, init) => {
    fetches.push({ url, ...init });
    if (failNext > 0) {
      failNext -= 1;
      return { ok: false, status: 500 };
    }
    return { ok: true, status: 200 };
  };
  world.env.WEBHOOK_SLEEP = async () => {};

  const created = await call(
    'POST',
    '/v1/webhooks',
    { url: 'https://hooks.example.test/rules' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(created.status, 200, created.text);
  assert.match(created.json.id, /^wh_/);
  assert.match(created.json.secret, /^whsec_/);
  const secret = created.json.secret;

  const dispatch = await call(
    'POST',
    '/v1/webhooks/dispatch',
    { from: '2026-01-01', to: '2026-07-01' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(dispatch.status, 200, dispatch.text);
  assert.equal(dispatch.json.deliveries.length, 1);
  const delivery = dispatch.json.deliveries[0];
  assert.equal(delivery.status, 'delivered');
  assert.equal(delivery.attempts, 3);
  assert.equal(fetches.length, 3);

  const signed = fetches[2];
  assert.equal(
    verifySignature(
      secret,
      signed.body,
      signed.headers['takehome-signature'],
    Math.floor(Date.parse('2026-09-16T00:00:00Z') / 1000),
    ),
    true,
  );
  const payload = JSON.parse(signed.body);
  assert.equal(payload.event, 'rule_set.changed');
  assert.deepEqual(payload.changed, ['BC', 'NL', 'PE']);

  const log = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(log.status, 200);
  assert.equal(log.json.deliveries.length, 1);
  assert.equal(log.json.deliveries[0].id, delivery.id);

  failNext = 0;
  const replay = await call(
    'POST',
    `/v1/webhooks/deliveries/${delivery.id}/replay`,
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(replay.status, 200, replay.text);
  assert.equal(replay.json.status, 'delivered');
  assert.equal(replay.json.attempts, 1);
  assert.notEqual(replay.json.id, delivery.id);
  assert.equal(replay.json.replay_of, delivery.id);
});

test('46. webhook routes require a key', async () => {
  const { status, json } = await call('POST', '/v1/webhooks', {
    url: 'https://hooks.example.test/rules',
  }, { authorization: null });
  assert.equal(status, 401);
  assert.equal(json.error.code, 'unauthorized');
});
