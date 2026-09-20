import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { handle } from '../src/handler.js';
import { nodeEngine } from '../src/engine-node.js';
import {
  MAX_BODY_BYTES,
  MAX_JSON_DEPTH,
  parseJsonStrict,
  BodyError,
} from '../src/json-body.js';
import { signStripeEvent, verifyStripeSignature } from '../src/stripe.js';
import { call, createWorld, ON_WEEKLY, request } from './helpers.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(here, '../../..');
const ADR = readFileSync(
  path.join(REPO, 'docs/ADR-005-metering-fail-closed.md'),
  'utf8',
);
const WORKFLOW = readFileSync(
  path.join(REPO, '.github/workflows/test.yml'),
  'utf8',
);

const LEAK =
  /(?:^|[^a-z])(?:wrangler|workerd|panicked)(?:[^a-z]|$)|\/home\/|\/usr\//;
const STACK = /\bat [\w.]+\s+\(/;

function assertTyped(status, json, text, code, httpStatus) {
  assert.equal(status, httpStatus, text);
  assert.equal(typeof json?.error, 'object', text);
  assert.equal(json.error.code, code, text);
  assert.equal(typeof json.error.message, 'string', text);
  assert.match(json.error.docs, /^https:\/\//, text);
  assert.deepEqual(Object.keys(json.error).sort(), ['code', 'docs', 'message']);
  assert.equal(json.employee, undefined);
  assert.equal(LEAK.test(text), false, text);
  assert.equal(STACK.test(text), false, text);
}

test('51. D1 metering outage fails closed with 503 and does not serve the calculation', async () => {
  assert.match(ADR, /Status\s*\n\s*\nAccepted/i);
  assert.match(ADR, /fail closed/i);
  assert.match(ADR, /503/);
  assert.match(ADR, /unmetered/i);

  const usageWorld = createWorld('developer');
  usageWorld.store.getUsage = () => {
    throw new Error('D1_ERROR network: usage query failed');
  };
  const usage = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${usageWorld.liveKey}` },
    usageWorld,
  );
  assertTyped(usage.status, usage.json, usage.text, 'store', 503);
  assert.equal(usage.text.includes('D1_ERROR'), false);
  assert.equal(usageWorld.store.getUsage === undefined, false);

  const billWorld = createWorld('developer');
  billWorld.store.addUsage = () => {
    throw new Error('D1_ERROR write: usage insert failed');
  };
  const billed = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${billWorld.liveKey}` },
    billWorld,
  );
  assertTyped(billed.status, billed.json, billed.text, 'store', 503);
  assert.equal(billWorld.store.getUsage(billWorld.account.id, '2026-09-01'), 0);
});

test('52. an engine panic is a typed 500 with no detail, is logged server-side, and does not take down the isolate', async () => {
  await call('GET', '/health');
  const world = createWorld();
  const panicking = {
    ...nodeEngine,
    calculate() {
      throw new Error('panicked at takehome_core::cpp: assertion failed');
    },
  };
  const lines = [];
  const original = console.log;
  console.log = (...args) => {
    lines.push(args.map(String).join(' '));
  };
  try {
    const first = await handle(
      request(
        'POST',
        '/v1/deductions',
        ON_WEEKLY,
        { authorization: `Bearer ${world.testKey}` },
      ),
      world.env,
      panicking,
    );
    const firstText = await first.text();
    const firstJson = JSON.parse(firstText);
    assertTyped(first.status, firstJson, firstText, 'engine', 500);
    assert.equal(firstText.includes('takehome_core'), false);
    assert.equal(firstText.includes('assertion failed'), false);
    const blob = lines.join('\n');
    assert.match(blob, /engine_panic|panicked at takehome_core/);
    assert.match(blob, /assertion failed/);

    const second = await handle(
      request(
        'POST',
        '/v1/deductions',
        ON_WEEKLY,
        { authorization: `Bearer ${world.testKey}` },
      ),
      world.env,
      panicking,
    );
    assert.equal(second.status, 500);
    const recovered = await handle(
      request(
        'POST',
        '/v1/deductions',
        ON_WEEKLY,
        { authorization: `Bearer ${world.testKey}` },
      ),
      world.env,
      nodeEngine,
    );
    assert.equal(recovered.status, 200);
  } finally {
    console.log = original;
  }
});

test('53. malformed, empty, wrong type, oversized, nested, and duplicate-key JSON are typed errors', async () => {
  const world = createWorld();
  const auth = { authorization: `Bearer ${world.testKey}` };

  const malformed = await call(
    'POST',
    '/v1/deductions',
    '{',
    { ...auth, 'content-type': 'application/json' },
    world,
  );
  assertTyped(malformed.status, malformed.json, malformed.text, 'malformed_json', 400);

  const empty = await call(
    'POST',
    '/v1/deductions',
    '',
    { ...auth, 'content-type': 'application/json' },
    world,
  );
  assertTyped(empty.status, empty.json, empty.text, 'malformed_json', 400);

  const wrongType = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { ...auth, 'content-type': 'text/plain' },
    world,
  );
  assertTyped(
    wrongType.status,
    wrongType.json,
    wrongType.text,
    'unsupported_media_type',
    415,
  );

  const started = Date.now();
  const huge = await call(
    'POST',
    '/v1/deductions',
    'x'.repeat(10 * 1024 * 1024),
    { ...auth, 'content-type': 'application/json' },
    world,
  );
  assert.ok(Date.now() - started < 2000, '10MB body hung');
  assertTyped(huge.status, huge.json, huge.text, 'payload_too_large', 413);
  assert.ok(MAX_BODY_BYTES < 10 * 1024 * 1024);

  const deepStarted = Date.now();
  const deep = `${'{"a":'.repeat(MAX_JSON_DEPTH + 1)}1${'}'.repeat(MAX_JSON_DEPTH + 1)}`;
  const nested = await call(
    'POST',
    '/v1/deductions',
    deep,
    { ...auth, 'content-type': 'application/json' },
    world,
  );
  assert.ok(Date.now() - deepStarted < 2000, 'deep JSON hung');
  assertTyped(nested.status, nested.json, nested.text, 'malformed_json', 400);

  const dup = await call(
    'POST',
    '/v1/deductions',
    '{"as_of":"2026-01-15","province":"ON","province":"QC","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}',
    { ...auth, 'content-type': 'application/json' },
    world,
  );
  assertTyped(dup.status, dup.json, dup.text, 'malformed_json', 400);
  assert.match(dup.json.error.message, /duplicate/i);

  parseJsonStrict('{"a":{"a":1}}');
  assert.throws(
    () => parseJsonStrict('{"a":1,"a":2}'),
    (err) => err instanceof BodyError && err.code === 'malformed_json',
  );
});

test('54. Quebec, out-of-range dates, and unknown rule versions are typed exceptional conditions', async () => {
  const quebec = await call('POST', '/v1/deductions', {
    as_of: '2026-01-15',
    province: 'QC',
    pay_period: 52,
    gross_pay: '1000.00',
  });
  assertTyped(quebec.status, quebec.json, quebec.text, 'jurisdiction_not_supported', 422);
  assert.match(quebec.json.error.message, /Quebec/);

  const range = await call('POST', '/v1/deductions', {
    ...ON_WEEKLY,
    as_of: '2025-12-31',
  });
  assertTyped(range.status, range.json, range.text, 'date_out_of_range', 400);

  const unknown = await call('GET', '/v1/rules/1999-01-01');
  assertTyped(unknown.status, unknown.json, unknown.text, 'not_found', 404);
  assert.match(unknown.json.error.message, /1999-01-01/);
});

test('55. billing webhook is dark; Stripe signature helpers stay fail-closed and warm', async () => {
  const world = createWorld();
  const dark = await call(
    'POST',
    '/v1/billing/webhook',
    { id: 'evt_should_not_apply', type: 'checkout.session.completed' },
    { 'content-type': 'application/json' },
    world,
  );
  assertTyped(dark.status, dark.json, dark.text, 'not_found', 404);
  assert.equal(world.store.getAccount(world.account.id).plan, 'developer');

  const secret = 'whsec_test_secret';
  const event = {
    id: 'evt_a10_replay',
    type: 'checkout.session.completed',
    data: {
      object: {
        client_reference_id: world.account.id,
        metadata: { plan: 'starter' },
      },
    },
  };
  const raw = JSON.stringify(event);
  const timestamp = Math.floor(Date.parse('2026-09-16T00:00:00Z') / 1000);
  const header = signStripeEvent(secret, raw, timestamp);
  assert.equal(
    verifyStripeSignature(secret, raw, header, timestamp),
    true,
  );
  assert.equal(
    verifyStripeSignature(
      secret,
      raw,
      't=1,v1=0000000000000000000000000000000000000000000000000000000000000000',
      timestamp,
    ),
    false,
  );

  // Store-level idempotency for the turn-on path (ADR-006 keeps this warm).
  assert.equal(world.store.hasStripeEvent(event.id), false);
  world.store.recordStripeEvent(event.id);
  assert.equal(world.store.hasStripeEvent(event.id), true);
  world.store.setPlan(world.account.id, 'starter');
  assert.equal(world.store.getAccount(world.account.id).plan, 'starter');
  // Replay of a recorded event id must not double-apply when billing returns.
  assert.equal(world.store.hasStripeEvent(event.id), true);
});

test('Gate 2. CI job security runs the exceptional-conditions tests', () => {
  assert.match(WORKFLOW, /^  security:\s*$/m);
  assert.match(WORKFLOW, /test:security|58-exceptions\.test\.js/);
  const security = WORKFLOW.split(/^  security:\s*$/m)[1] ?? '';
  const next = security.split(/^  [A-Za-z0-9_-]+:\s*$/m)[0];
  assert.match(next, /^\s+permissions:\s*$/m);
  assert.match(next, /^\s+contents:\s+read\s*$/m);
});
