import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { PLANS, cadLabel, livePlan } from '../src/plans.js';
import { call, createWorld } from './helpers.js';

const SITE = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../../web/site',
);

test('24. published tiers are CAD integer cents, not floats', () => {
  assert.equal(PLANS.currency, 'CAD');
  for (const plan of PLANS.live) {
    assert.equal(typeof plan.cad_cents, 'number');
    assert.equal(plan.cad_cents % 1, 0);
  }
  assert.equal(cadLabel(4900), '$49 CAD');
  assert.equal(cadLabel(14900), '$149 CAD');
  assert.equal(cadLabel(39900), '$399 CAD');
  assert.equal(cadLabel(0), '$0 CAD');
});

test('24. live checkout is deferred with billing_unavailable (ADR-006)', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/billing/checkout',
    {
      plan: 'starter',
      success_url: 'https://takehome.gautamkhosla.com/signup/verify/?ok=1',
      cancel_url: 'https://takehome.gautamkhosla.com/pricing/',
    },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 501);
  assert.equal(json.error.code, 'billing_unavailable');
  assert.equal(world.stripeSessions.length, 0);
  assert.equal(livePlan('developer').calculations, 100_000);
});

test('24. checkout never offers contact sales', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/billing/checkout',
    { plan: 'enterprise' },
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 501);
  assert.equal(json.error.code, 'billing_unavailable');
  assert.doesNotMatch(json.error.message, /contact sales/i);
});

test('24. billing webhook is not listening (404)', async () => {
  const world = createWorld();
  const { status, json } = await call(
    'POST',
    '/v1/billing/webhook',
    {
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

test('24. site copy has no contact-sales wedge and states free early access', () => {
  const files = [
    'src/pages/pricing.astro',
    'src/pages/signup/index.astro',
    'src/pages/signup/verify.astro',
  ];
  for (const rel of files) {
    const text = readFileSync(path.join(SITE, rel), 'utf8');
    assert.doesNotMatch(text, /contact sales/i, rel);
    assert.doesNotMatch(text, /talk to sales/i, rel);
    assert.doesNotMatch(text, /book a demo/i, rel);
    assert.doesNotMatch(text, /request a demo/i, rel);
  }
  const pricing = readFileSync(path.join(SITE, 'src/pages/pricing.astro'), 'utf8');
  assert.match(pricing, /free during early access/i);
  assert.match(pricing, /billing_unavailable/);
  assert.match(pricing, /free\.calculations/);
  const plans = JSON.parse(
    readFileSync(path.join(SITE, 'src/data/plans.json'), 'utf8'),
  );
  assert.equal(
    plans.live.find((p) => p.id === 'developer').calculations,
    100_000,
  );
});
