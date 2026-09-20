import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { call, createWorld } from './helpers.js';
import { livePlan, PLANS } from '../src/plans.js';
import {
  constructStripeEvent,
  signStripeEvent,
  verifyStripeSignature,
} from '../src/stripe.js';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const WRANGLER = readFileSync(path.join(ROOT, '../wrangler.toml'), 'utf8');
const STRIPE_SRC = readFileSync(path.join(ROOT, '../src/stripe.js'), 'utf8');
const HANDLER = readFileSync(path.join(ROOT, '../src/handler.js'), 'utf8');
const FREE_LIMIT = livePlan('developer').calculations;

test('60. POST /v1/billing/checkout is a typed 501 billing_unavailable', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/billing/checkout',
    { plan: 'starter' },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 501);
  assert.equal(json.error.code, 'billing_unavailable');
  assert.match(json.error.message, /free/i);
  assert.match(json.error.docs, /pricing/);
  assert.equal(json.url, undefined);
  assert.equal(world.stripeSessions.length, 0);
});

test('60. checkout without a live key stays 401 (not a broken Stripe redirect)', async () => {
  const missing = await call('POST', '/v1/billing/checkout', { plan: 'starter' });
  assert.equal(missing.status, 401);
  assert.equal(missing.json.error.code, 'unauthorized');

  const world = createWorld();
  const testKey = await call(
    'POST',
    '/v1/billing/checkout',
    { plan: 'starter' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(testKey.status, 401);
});

test('60. POST /v1/billing/webhook is 404; no signed-event listener', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/billing/webhook',
    {
      id: 'evt_deferred',
      type: 'checkout.session.completed',
      data: {
        object: {
          client_reference_id: world.account.id,
          metadata: { plan: 'starter' },
        },
      },
    },
    {},
    world,
  );
  assert.equal(status, 404);
  assert.equal(json.error.code, 'not_found');
  assert.equal(world.store.getAccount(world.account.id).plan, 'developer');
});

test('60. production wrangler config references no Stripe secret', () => {
  assert.doesNotMatch(WRANGLER, /STRIPE_/);
  assert.doesNotMatch(WRANGLER, /sk_live_|sk_test_|whsec_/);
  // Top-level production binding stays takehome, not staging.
  assert.match(WRANGLER, /database_name\s*=\s*"takehome"/);
  assert.match(WRANGLER, /database_id\s*=\s*"3cfd409e-441b-434a-a2a0-1192562156e8"/);
});

test('60. handler live routes do not call stripeClient; stripe.js keeps ADR pointer', () => {
  assert.doesNotMatch(HANDLER, /stripeClient\s*\(/);
  assert.doesNotMatch(HANDLER, /from '\.\/stripe\.js'/);
  assert.match(STRIPE_SRC, /ADR-006/);
});

test('60. free early-access quota is explicit and above the old 1,000 developer cap', () => {
  assert.equal(FREE_LIMIT, 100_000);
  assert.equal(PLANS.live.find((p) => p.id === 'developer').calculations, 100_000);
  assert.equal(livePlan('starter').calculations, 25_000);
  assert.equal(livePlan('growth').calculations, 150_000);
  assert.equal(livePlan('business').calculations, 1_000_000);
});

test('60. Stripe signature helpers still verify and reject (kept for the turn-on)', () => {
  const secret = 'whsec_deferred_keep_warm';
  const raw = JSON.stringify({
    id: 'evt_keep_warm',
    type: 'checkout.session.completed',
    data: { object: { metadata: { plan: 'starter' } } },
  });
  const t = Math.floor(Date.parse('2026-09-16T00:00:00Z') / 1000);
  const header = signStripeEvent(secret, raw, t);
  assert.equal(verifyStripeSignature(secret, raw, header, t), true);
  assert.equal(
    verifyStripeSignature(secret, raw, 't=1,v1=00', t),
    false,
  );
  const event = constructStripeEvent(raw, header, secret, t);
  assert.equal(event.id, 'evt_keep_warm');
});
