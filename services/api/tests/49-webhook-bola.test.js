import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { addAccount, call, createWorld, newStore } from './helpers.js';
import { OBJECT_ID_ROUTES } from '../src/object-routes.js';
import { DELIVERY_ID_PATTERN, WEBHOOK_ID_PATTERN } from '../src/keys.js';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

function fixtureId(route, fixtures) {
  return route.idKind === 'delivery' ? fixtures.deliveryId : fixtures.webhookId;
}

async function seedTwoAccounts() {
  const world = createWorld();
  world.env.WEBHOOK_SLEEP = async () => {};
  world.env.WEBHOOK_FETCH = async () => ({ ok: true, status: 200 });
  const other = addAccount(world);

  const created = await call(
    'POST',
    '/v1/webhooks',
    { url: 'https://hooks.example.test/rules' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(created.status, 200, created.text);

  const dispatched = await call(
    'POST',
    '/v1/webhooks/dispatch',
    { from: '2026-01-01', to: '2026-07-01' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(dispatched.status, 200, dispatched.text);
  assert.equal(dispatched.json.deliveries.length, 1);

  return {
    world,
    ownerKey: world.testKey,
    otherKey: other.testKey,
    webhookId: created.json.id,
    deliveryId: dispatched.json.deliveries[0].id,
  };
}

test('49. webhook and delivery IDs are 128-bit random, not sequential', async () => {
  const world = createWorld();
  world.env.WEBHOOK_SLEEP = async () => {};
  world.env.WEBHOOK_FETCH = async () => ({ ok: true, status: 200 });
  const ids = [];
  for (let i = 0; i < 2; i += 1) {
    const created = await call(
      'POST',
      '/v1/webhooks',
      { url: 'https://hooks.example.test/rules' },
      { authorization: `Bearer ${world.testKey}` },
      world,
    );
    assert.equal(created.status, 200, created.text);
    assert.match(created.json.id, WEBHOOK_ID_PATTERN);
    assert.equal(/^wh_\d+$/.test(created.json.id), false);
    ids.push(created.json.id);
  }
  assert.notEqual(ids[0], ids[1]);

  const dispatched = await call(
    'POST',
    '/v1/webhooks/dispatch',
    { from: '2026-01-01', to: '2026-07-01' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(dispatched.status, 200, dispatched.text);
  for (const delivery of dispatched.json.deliveries) {
    assert.match(delivery.id, DELIVERY_ID_PATTERN);
    assert.equal(/^whd_\d+$/.test(delivery.id), false);
    assert.match(delivery.endpoint_id, WEBHOOK_ID_PATTERN);
  }
});

test('49. GET /v1/webhooks/deliveries is scoped to the authenticated account', async () => {
  const fx = await seedTwoAccounts();
  const ownerList = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${fx.ownerKey}` },
    fx.world,
  );
  const otherList = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  assert.equal(ownerList.status, 200);
  assert.equal(ownerList.json.deliveries.length, 1);
  assert.equal(ownerList.json.deliveries[0].id, fx.deliveryId);
  assert.equal(otherList.status, 200);
  assert.deepEqual(otherList.json.deliveries, []);
});

test('49. GET /v1/webhooks does not list another account\'s endpoints', async () => {
  const fx = await seedTwoAccounts();
  const ownerList = await call(
    'GET',
    '/v1/webhooks',
    undefined,
    { authorization: `Bearer ${fx.ownerKey}` },
    fx.world,
  );
  const otherList = await call(
    'GET',
    '/v1/webhooks',
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  assert.equal(ownerList.status, 200);
  assert.equal(ownerList.json.webhooks.length, 1);
  assert.equal(ownerList.json.webhooks[0].id, fx.webhookId);
  assert.equal(otherList.status, 200);
  assert.deepEqual(otherList.json.webhooks, []);
});

test('49. listDeliveries is scoped by account in the store query, not after a broad read', () => {
  const memory = readFileSync(path.join(ROOT, '../src/store-memory.js'), 'utf8');
  const d1 = readFileSync(path.join(ROOT, '../src/store-d1.js'), 'utf8');
  const handler = readFileSync(path.join(ROOT, '../src/handler.js'), 'utf8');

  const memFn = memory.match(/listDeliveries\(accountId\) \{[\s\S]*?\n  \}/);
  assert.ok(memFn, 'MemoryStore.listDeliveries must exist');
  assert.equal(memFn[0].includes('this.deliveries.values()'), false);
  assert.match(memFn[0], /deliveriesByAccount/);

  const d1Fn = d1.match(/async listDeliveries\(accountId\) \{[\s\S]*?\n  \}/);
  assert.ok(d1Fn, 'D1Store.listDeliveries must exist');
  assert.match(d1Fn[0], /WHERE[\s\S]*account_id = \?/);

  assert.match(handler, /listDeliveries\(auth\.account\.id\)/);
  assert.doesNotMatch(handler, /listDeliveries\(\s*\)/);
});

test('49. store lookups take accountId and miss on a foreign account', () => {
  const store = newStore();
  const owner = store.createAccount({
    email: 'owner@example.com',
    email_verified: true,
  });
  const other = store.createAccount({
    email: 'other@example.com',
    email_verified: true,
  });
  store.insertWebhook({
    id: 'wh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    account_id: owner.id,
    url: 'https://hooks.example.test/rules',
    secret: 'whsec_test',
    created_at: '2026-09-16T00:00:00Z',
  });
  store.insertDelivery({
    id: 'whd_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
    account_id: owner.id,
    endpoint_id: 'wh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    event: 'rule_set.changed',
    payload: '{}',
    status: 'delivered',
    attempts: 1,
    last_error: null,
    last_http_status: 200,
    replay_of: null,
    created_at: '2026-09-16T00:00:00Z',
    delivered_at: '2026-09-16T00:00:00Z',
  });
  assert.equal(
    store.getWebhook('wh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', other.id),
    null,
  );
  assert.equal(
    store.getDelivery('whd_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', other.id),
    null,
  );
  assert.equal(
    store.getWebhook('wh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', owner.id)?.id,
    'wh_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
  );
  assert.equal(
    store.getDelivery('whd_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', owner.id)?.id,
    'whd_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
  );
  assert.deepEqual(store.listDeliveries(other.id), []);
  assert.equal(store.listDeliveries(owner.id).length, 1);
});

test('49. handler dispatches object-ID routes from the registered table', () => {
  const handler = readFileSync(path.join(ROOT, '../src/handler.js'), 'utf8');
  assert.match(handler, /matchObjectIdRoute/);
  assert.match(handler, /OBJECT_ID_HANDLERS/);
  assert.ok(OBJECT_ID_ROUTES.length > 0);
  for (const route of OBJECT_ID_ROUTES) {
    assert.match(handler, new RegExp(`${route.name}\\s*:`));
    assert.equal(['webhook', 'delivery'].includes(route.idKind), true, route.name);
    assert.equal(typeof route.path, 'function');
    assert.equal(typeof route.match, 'function');
  }
});

test('49. cross-account replay matches an unknown id (no existence leak) and does not act', async () => {
  const fx = await seedTwoAccounts();
  const before = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${fx.ownerKey}` },
    fx.world,
  );
  const cross = await call(
    'POST',
    `/v1/webhooks/deliveries/${fx.deliveryId}/replay`,
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  const missing = await call(
    'POST',
    '/v1/webhooks/deliveries/whd_ffffffffffffffffffffffffffffffff/replay',
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  assert.equal(cross.status, 404);
  assert.equal(missing.status, 404);
  assert.equal(cross.json.error.code, 'not_found');
  assert.equal(missing.json.error.code, 'not_found');
  assert.notEqual(cross.status, 403);
  assert.doesNotMatch(cross.json.error.message, /account|forbidden|permission/i);

  const after = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${fx.ownerKey}` },
    fx.world,
  );
  assert.equal(after.json.deliveries.length, before.json.deliveries.length);

  const otherList = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  assert.deepEqual(otherList.json.deliveries, []);
});

test('49. cross-account DELETE does not remove the webhook', async () => {
  const fx = await seedTwoAccounts();
  const cross = await call(
    'DELETE',
    `/v1/webhooks/${fx.webhookId}`,
    undefined,
    { authorization: `Bearer ${fx.otherKey}` },
    fx.world,
  );
  assert.equal(cross.status, 404);
  assert.equal(cross.json.error.code, 'not_found');
  assert.notEqual(cross.status, 403);
  const still = await call(
    'GET',
    `/v1/webhooks/${fx.webhookId}`,
    undefined,
    { authorization: `Bearer ${fx.ownerKey}` },
    fx.world,
  );
  assert.equal(still.status, 200);
  assert.equal(still.json.id, fx.webhookId);
  assert.equal(still.json.secret, undefined);
});

for (const route of OBJECT_ID_ROUTES) {
  test(`49. owner ${route.method} ${route.pattern} returns 200`, async () => {
    const fx = await seedTwoAccounts();
    const { status, json } = await call(
      route.method,
      route.path(fixtureId(route, fx)),
      undefined,
      { authorization: `Bearer ${fx.ownerKey}` },
      fx.world,
    );
    assert.equal(status, 200, JSON.stringify(json));
  });

  test(`49. other account ${route.method} ${route.pattern} returns 404, not 403`, async () => {
    const fx = await seedTwoAccounts();
    const { status, json } = await call(
      route.method,
      route.path(fixtureId(route, fx)),
      undefined,
      { authorization: `Bearer ${fx.otherKey}` },
      fx.world,
    );
    assert.equal(status, 404, JSON.stringify(json));
    assert.notEqual(status, 403);
    assert.equal(json.error.code, 'not_found');
    assert.doesNotMatch(json.error.message, /forbidden|permission|not yours/i);
  });
}
