import assert from 'node:assert/strict';
import { test } from 'node:test';
import { verificationEmailBodies } from '../src/verify-mail.js';
import { sendMail } from '../src/mail.js';

const URL =
  'https://takehome.gautamkhosla.com/signup/verify/?token=abcdef0123456789abcdef0123456789';
const EXPIRY = '2026-09-21T17:00:00Z';

test('61. verification email text and html carry the same URL exactly once', () => {
  const { text, html } = verificationEmailBodies({
    verifyUrl: URL,
    expiresAt: EXPIRY,
  });
  assert.equal([...text.matchAll(new RegExp(URL.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'))].length, 1);
  assert.equal([...html.matchAll(new RegExp(URL.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'))].length, 1);
  assert.match(text, /T4127/);
  assert.match(text, /test key/);
  assert.match(text, /live key/);
  assert.match(text, /expires at 2026-09-21T17:00:00Z UTC/);
  assert.match(text, /single use|works once/i);
  assert.match(html, /<a href="/);
  assert.doesNotMatch(text, /https?:\/\/[^/\s]*(track|pixel|open\.|click\.)/i);
  assert.doesNotMatch(html, /https?:\/\/[^/\s]*(track|pixel|open\.|click\.)/i);
  assert.doesNotMatch(html, /<img\b/i);
  assert.ok(text.split('\n').length <= 12);
});

test('61. Resend payload includes html and reply_to', async () => {
  const calls = [];
  await sendMail(
    {
      RESEND_API_KEY: 're_test',
      MAIL_FROM: 'Takehome <noreply@gautamkhosla.com>',
      MAIL_REPLY_TO: 'developwith.gt@gmail.com',
      fetch: async (url, init) => {
        calls.push({ url, body: JSON.parse(init.body) });
        return new Response(JSON.stringify({ id: 'email_1' }), { status: 200 });
      },
    },
    {
      to: 'ada@example.com',
      subject: 'Verify your Takehome email',
      text: `body ${URL}`,
      html: `<p><a href="${URL}">${URL}</a></p>`,
    },
  );
  assert.equal(calls.length, 1);
  assert.equal(calls[0].body.reply_to, 'developwith.gt@gmail.com');
  assert.equal(calls[0].body.html.includes(URL), true);
  assert.equal(calls[0].body.text.includes(URL), true);
});
