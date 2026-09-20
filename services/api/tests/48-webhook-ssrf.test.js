import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld } from './helpers.js';
import {
  DELIVERY_TIMEOUT_MS,
  MAX_RESPONSE_BODY_BYTES,
  deliverWithRetry,
} from '../src/webhooks.js';
import {
  UnsafeWebhookUrlError,
  assertSafeWebhookUrl,
  isBlockedAddress,
} from '../src/ssrf.js';

const PUBLIC_IP = '203.0.113.10';
const ATTACKER_BODY = 'ATTACKER_CONTROLLED_BODY_FROM_METADATA';

function authWorld(resolveImpl) {
  const world = createWorld();
  world.env.WEBHOOK_SLEEP = async () => {};
  if (resolveImpl) {
    world.env.WEBHOOK_RESOLVE = resolveImpl;
  }
  return world;
}

async function register(world, url) {
  return call(
    'POST',
    '/v1/webhooks',
    { url },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
}

async function dispatch(world) {
  return call(
    'POST',
    '/v1/webhooks/dispatch',
    { from: '2026-01-01', to: '2026-07-01' },
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
}

test('48. constants cap delivery time and body read', () => {
  assert.ok(DELIVERY_TIMEOUT_MS > 0);
  assert.ok(DELIVERY_TIMEOUT_MS <= 10_000);
  assert.ok(MAX_RESPONSE_BODY_BYTES > 0);
  assert.ok(MAX_RESPONSE_BODY_BYTES <= 16_384);
});

test('48. reject non-https schemes at registration', async () => {
  const schemes = [
    'http://hooks.example.test/rules',
    'file:///etc/passwd',
    'gopher://hooks.example.test/1',
    'ftp://hooks.example.test/rules',
    'data:text/plain,hello',
    'javascript:alert(1)',
  ];
  for (const url of schemes) {
    const world = authWorld(async () => [PUBLIC_IP]);
    const { status, json } = await register(world, url);
    assert.equal(status, 400, url);
    assert.equal(json.error.code, 'invalid_request', url);
  }
});

test('48. reject by resolved IP, not by hostname string matching', async () => {
  const blocked = [
    '127.0.0.1',
    '10.1.2.3',
    '172.16.0.1',
    '172.31.255.1',
    '192.168.0.1',
    '169.254.169.254',
    '0.0.0.1',
    '100.64.0.1',
    '192.0.0.1',
    '198.18.0.1',
    '224.0.0.1',
    '240.0.0.1',
    '::1',
    'fc00::1',
    'fd12:3456::1',
    'fe80::1',
    '::ffff:127.0.0.1',
    '::ffff:8.8.8.8',
    '::ffff:7f00:1',
  ];
  for (const ip of blocked) {
    await assert.rejects(
      () =>
        assertSafeWebhookUrl('https://hooks.example.test/rules', {
          resolve: async () => [ip],
        }),
      (err) => {
        assert.equal(err instanceof UnsafeWebhookUrlError, true, ip);
        assert.match(err.message, /blocked address/i, ip);
        return true;
      },
    );
    assert.equal(isBlockedAddress(ip), true, ip);
    const world = authWorld(async () => [ip]);
    const { status, json } = await register(
      world,
      'https://hooks.example.test/rules',
    );
    assert.equal(status, 400, ip);
    assert.equal(json.error.code, 'invalid_request', ip);
  }
});

test('48. allow a hostname that resolves only to a public unicast address', async () => {
  const allowed = ['8.8.8.8', PUBLIC_IP, '172.32.0.1', '100.63.255.255'];
  for (const ip of allowed) {
    assert.equal(isBlockedAddress(ip), false, ip);
    await assertSafeWebhookUrl('https://hooks.example.test/rules', {
      resolve: async () => [ip],
    });
  }
  const world = authWorld(async () => [PUBLIC_IP]);
  const { status } = await register(world, 'https://hooks.example.test/rules');
  assert.equal(status, 200);
});

test('48. reject if any resolved address is blocked', async () => {
  await assert.rejects(
    () =>
      assertSafeWebhookUrl('https://hooks.example.test/rules', {
        resolve: async () => [PUBLIC_IP, '127.0.0.1'],
      }),
    UnsafeWebhookUrlError,
  );
});

test('48. reject hostnames that are bare IPs in decimal, octal, or hex', async () => {
  const urls = [
    'https://2130706433/rules',
    'https://0177.0.0.1/rules',
    'https://0x7f000001/rules',
    'https://0x7f.0x0.0x0.0x1/rules',
    'https://127.0.0.1/rules',
    'https://127.1/rules',
    'https://[::1]/rules',
    'https://[::ffff:127.0.0.1]/rules',
  ];
  for (const url of urls) {
    await assert.rejects(
      () => assertSafeWebhookUrl(url, { resolve: async () => [PUBLIC_IP] }),
      UnsafeWebhookUrlError,
    );
    const world = authWorld(async () => [PUBLIC_IP]);
    const { status, json } = await register(world, url);
    assert.equal(status, 400, url);
    assert.equal(json.error.code, 'invalid_request', url);
  }
});

test('48. reject localhost, *.localhost, *.internal, *.local, and names with no dot', async () => {
  const urls = [
    'https://localhost/rules',
    'https://foo.localhost/rules',
    'https://metadata.google.internal/rules',
    'https://printer.local/rules',
    'https://webhook/rules',
    'https://internal/rules',
    'https://local/rules',
  ];
  for (const url of urls) {
    await assert.rejects(
      () => assertSafeWebhookUrl(url, { resolve: async () => [PUBLIC_IP] }),
      UnsafeWebhookUrlError,
    );
    const world = authWorld(async () => [PUBLIC_IP]);
    const { status, json } = await register(world, url);
    assert.equal(status, 400, url);
    assert.equal(json.error.code, 'invalid_request', url);
  }
});

test('48. DNS rebinding: re-validate at dispatch and refuse the fetch', async () => {
  let lookups = 0;
  const world = authWorld(async () => {
    lookups += 1;
    return lookups === 1 ? [PUBLIC_IP] : ['127.0.0.1'];
  });
  const fetches = [];
  world.env.WEBHOOK_FETCH = async (url, init) => {
    fetches.push({ url, init });
    return { ok: true, status: 200 };
  };

  const created = await register(world, 'https://hooks.example.test/rules');
  assert.equal(created.status, 200, created.text);
  assert.equal(lookups, 1);

  const sent = await dispatch(world);
  assert.equal(sent.status, 200, sent.text);
  assert.equal(sent.json.deliveries.length, 1);
  assert.equal(sent.json.deliveries[0].status, 'failed');
  assert.match(
    sent.json.deliveries[0].last_error,
    /blocked address/i,
  );
  assert.equal(fetches.length, 0);
  assert.equal(lookups, 2);
});

test('48. webhook delivery does not follow redirects', async () => {
  const world = authWorld(async () => [PUBLIC_IP]);
  const fetches = [];
  world.env.WEBHOOK_FETCH = async (url, init) => {
    fetches.push({ url, redirect: init.redirect, headers: init.headers });
    return {
      ok: false,
      status: 302,
      headers: {
        get(name) {
          return name.toLowerCase() === 'location'
            ? 'http://169.254.169.254/latest/meta-data/'
            : null;
        },
      },
    };
  };

  const created = await register(world, 'https://hooks.example.test/rules');
  assert.equal(created.status, 200, created.text);
  const sent = await dispatch(world);
  assert.equal(sent.status, 200, sent.text);
  const delivery = sent.json.deliveries[0];
  assert.equal(delivery.status, 'failed');
  assert.equal(delivery.last_error, 'HTTP 302');
  assert.equal(fetches.length, 1);
  assert.equal(fetches[0].url, 'https://hooks.example.test/rules');
  assert.equal(fetches[0].redirect, 'manual');
});

test('48. a hanging webhook endpoint is aborted by the delivery timeout', async () => {
  const result = await deliverWithRetry({
    url: 'https://hooks.example.test/hang',
    body: '{"event":"rule_set.changed"}',
    secret: 'whsec_test',
    timestampSec: 1_758_000_000,
    timeoutMs: 25,
    fetchImpl: async (_url, init) => {
      assert.ok(init.signal, 'delivery fetch must pass an AbortSignal');
      await new Promise((_, reject) => {
        init.signal.addEventListener('abort', () => {
          reject(Object.assign(new Error('aborted'), { name: 'AbortError' }));
        });
      });
    },
    sleep: async () => {},
  });
  assert.equal(result.ok, false);
  assert.match(result.error, /timed out/i);
  assert.equal(result.status, null);
});

test('48. delivery reads at most MAX_RESPONSE_BODY_BYTES then cancels', async () => {
  let cancelled = false;
  let enqueued = 0;
  const chunk = new Uint8Array(1024).fill(65);
  const body = new ReadableStream({
    pull(controller) {
      enqueued += chunk.byteLength;
      controller.enqueue(chunk);
    },
    cancel() {
      cancelled = true;
    },
  });
  const result = await deliverWithRetry({
    url: 'https://hooks.example.test/rules',
    body: '{"event":"rule_set.changed"}',
    secret: 'whsec_test',
    timestampSec: 1_758_000_000,
    fetchImpl: async () => ({ ok: true, status: 200, body }),
    sleep: async () => {},
  });
  assert.equal(result.ok, true);
  assert.equal(cancelled, true);
  assert.ok(enqueued <= MAX_RESPONSE_BODY_BYTES + chunk.byteLength);
});

test('48. delivery log stores the response status only, never the body', async () => {
  const world = authWorld(async () => [PUBLIC_IP]);
  let textCalled = false;
  world.env.WEBHOOK_FETCH = async () => ({
    ok: false,
    status: 500,
    async text() {
      textCalled = true;
      return ATTACKER_BODY;
    },
    async json() {
      textCalled = true;
      return { secret: ATTACKER_BODY };
    },
  });

  const created = await register(world, 'https://hooks.example.test/rules');
  assert.equal(created.status, 200, created.text);
  const sent = await dispatch(world);
  assert.equal(sent.status, 200, sent.text);
  const delivery = sent.json.deliveries[0];
  assert.equal(delivery.status, 'failed');
  assert.equal(delivery.last_error, 'HTTP 500');
  assert.equal(textCalled, false);

  const log = await call(
    'GET',
    '/v1/webhooks/deliveries',
    undefined,
    { authorization: `Bearer ${world.testKey}` },
    world,
  );
  assert.equal(log.status, 200);
  const dumped = JSON.stringify(log.json);
  assert.equal(dumped.includes(ATTACKER_BODY), false);
  assert.equal(delivery.last_error.includes(ATTACKER_BODY), false);

  const stored = [...world.store.deliveries.values()][0];
  assert.equal(stored.last_http_status, 500);
  assert.equal(JSON.stringify(stored).includes(ATTACKER_BODY), false);
});
