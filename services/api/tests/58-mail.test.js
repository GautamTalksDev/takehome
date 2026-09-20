import assert from 'node:assert/strict';
import { test } from 'node:test';
import { call, createWorld, ON_WEEKLY, newStore, BASE_ENV } from './helpers.js';
import { sendMail, MailTransportError } from '../src/mail.js';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const WRANGLER = readFileSync(
  path.join(path.dirname(fileURLToPath(import.meta.url)), '../wrangler.toml'),
  'utf8',
);

test('58. MAILBOX sink still receives mail without calling Resend', async () => {
  const mailbox = [];
  const fetches = [];
  await sendMail(
    { MAILBOX: mailbox, fetch: async (...args) => { fetches.push(args); } },
    {
      to: 'ada@example.com',
      subject: 'Verify your Takehome email',
      text: 'Verify: https://takehome.gautamkhosla.com/signup/verify/?token=abc',
      token: 'abc',
    },
  );
  assert.equal(mailbox.length, 1);
  assert.equal(mailbox[0].to, 'ada@example.com');
  assert.equal(fetches.length, 0);
});

test('58. Resend transport posts the expected shape and keeps the token in text', async () => {
  const calls = [];
  const env = {
    RESEND_API_KEY: 're_test_key',
    MAIL_FROM: 'Takehome <noreply@mail.gautamkhosla.com>',
    fetch: async (url, init) => {
      calls.push({ url, init });
      return new Response(JSON.stringify({ id: 'email_test' }), { status: 200 });
    },
  };
  const token = 'a'.repeat(32);
  const text = `Verify and receive API keys: https://takehome.gautamkhosla.com/signup/verify/?token=${token}`;
  await sendMail(env, {
    to: 'ada@example.com',
    subject: 'Verify your Takehome email',
    text,
    token,
  });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, 'https://api.resend.com/emails');
  assert.equal(calls[0].init.method, 'POST');
  assert.equal(calls[0].init.headers.authorization, 'Bearer re_test_key');
  assert.equal(calls[0].init.headers['content-type'], 'application/json');
  const body = JSON.parse(calls[0].init.body);
  assert.equal(body.from, 'Takehome <noreply@mail.gautamkhosla.com>');
  assert.deepEqual(body.to, ['ada@example.com']);
  assert.equal(body.subject, 'Verify your Takehome email');
  assert.equal(body.text, text);
  assert.match(body.text, new RegExp(token));
  assert.equal(body.html, undefined);
});

test('58. missing RESEND_API_KEY without MAILBOX throws MailTransportError', async () => {
  await assert.rejects(
    () =>
      sendMail(
        { MAIL_FROM: 'Takehome <noreply@mail.gautamkhosla.com>' },
        { to: 'ada@example.com', subject: 'x', text: 'y' },
      ),
    (err) => err instanceof MailTransportError,
  );
});

test('58. CR/LF in to or subject is rejected before transport', async () => {
  const mailbox = [];
  await sendMail(
    { MAILBOX: mailbox },
    { to: 'ada@example.com\nbcc:evil@x.com', subject: 'ok', text: 'y' },
  );
  assert.equal(mailbox.length, 0);
  await sendMail(
    { MAILBOX: mailbox },
    { to: 'ada@example.com', subject: 'ok\nBcc: evil', text: 'y' },
  );
  assert.equal(mailbox.length, 0);
});

test('58. signup without MAILBOX or RESEND_API_KEY fails closed with 503', async () => {
  const store = newStore();
  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'no-transport@example.com' },
    {},
    { env: { ...BASE_ENV, STORE: store }, testKey: null },
  );
  assert.equal(signed.status, 503);
  assert.equal(signed.json.error?.code, 'mail');
  assert.equal(signed.json.verify_url, undefined);
});

test('58. signup via Resend stub succeeds, token only in outbound text', async () => {
  const store = newStore();
  const calls = [];
  const env = {
    ...BASE_ENV,
    STORE: store,
    RESEND_API_KEY: 're_test_key',
    MAIL_FROM: 'Takehome <noreply@mail.gautamkhosla.com>',
    fetch: async (url, init) => {
      calls.push({ url, body: JSON.parse(init.body) });
      return new Response(JSON.stringify({ id: 'email_1' }), { status: 200 });
    },
  };
  const signed = await call(
    'POST',
    '/v1/signup',
    { email: 'resend@example.com' },
    {},
    { env, testKey: null },
  );
  assert.equal(signed.status, 200);
  assert.equal(signed.json.verify_url, undefined);
  assert.doesNotMatch(JSON.stringify(signed.json), /https?:\/\//i);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, 'https://api.resend.com/emails');
  assert.match(calls[0].body.text, /\/signup\/verify\/\?token=[0-9a-f]{32}/);
  const token = new URL(
    calls[0].body.text.match(/https?:\/\/\S+/)[0],
  ).searchParams.get('token');
  const verified = await call(
    'GET',
    `/v1/signup/verify?token=${token}`,
    undefined,
    {},
    { env, testKey: null },
  );
  assert.equal(verified.status, 200);
  assert.match(verified.json.test_key, /^np_test_/);
});

test('58. quota 402 alert goes through Resend to ALERT_EMAIL', async () => {
  const world = createWorld('developer');
  delete world.env.MAILBOX;
  world.mailbox.length = 0;
  const calls = [];
  world.env.ALERT_EMAIL = 'ops@takehome.example';
  world.env.RESEND_API_KEY = 're_test_key';
  world.env.MAIL_FROM = 'Takehome <noreply@mail.gautamkhosla.com>';
  world.env.fetch = async (url, init) => {
    calls.push({ url, body: JSON.parse(init.body) });
    return new Response(JSON.stringify({ id: 'alert_1' }), { status: 200 });
  };
  world.store.setUsage(world.account.id, '2026-09-01', 100_000);
  const { status } = await call(
    'POST',
    '/v1/deductions',
    ON_WEEKLY,
    { authorization: `Bearer ${world.liveKey}` },
    world,
  );
  assert.equal(status, 402);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].body.to[0], 'ops@takehome.example');
  assert.match(calls[0].body.subject, /quota_402/);
  assert.doesNotMatch(JSON.stringify(calls[0].body), /gross_pay/);
});

test('58. production wrangler.toml does not embed RESEND_API_KEY', () => {
  assert.doesNotMatch(WRANGLER, /^\s*RESEND_API_KEY\s*=/m);
  assert.match(WRANGLER, /MAIL_FROM/);
});
