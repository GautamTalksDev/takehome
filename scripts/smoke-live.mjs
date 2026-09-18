#!/usr/bin/env node
/**
 * M3 deploy smoke against the live origin, not localhost.
 * One calculator page computes, one authenticated API call succeeds,
 * /conformance renders all 164 disagreements.
 */
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(
  path.join(path.dirname(fileURLToPath(import.meta.url)), '../web/site/package.json'),
);
const { chromium } = require('playwright');

const ORIGIN = process.env.LIVE_ORIGIN ?? 'https://takehome.gautamkhosla.com';

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

async function json(method, path, body, headers = {}) {
  const response = await fetch(`${ORIGIN}${path}`, {
    method,
    headers: {
      'content-type': 'application/json',
      ...headers,
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    fail(`${method} ${path} -> ${response.status} non-JSON: ${text.slice(0, 200)}`);
  }
  return { status: response.status, body: parsed };
}

const home = await fetch(`${ORIGIN}/`);
if (!home.ok) fail(`GET / -> ${home.status}`);
const homeHtml = await home.text();
if (!homeHtml.includes('Takehome')) fail('home page missing Takehome');

const signup = await json('POST', '/v1/signup', {
  email: `smoke-${Date.now()}@example.com`,
});
if (signup.status !== 200) fail(`/v1/signup -> ${signup.status} ${JSON.stringify(signup.body)}`);
const verifyUrl = signup.body.verify_url;
if (!verifyUrl) fail('signup did not echo verify_url');
const token = new URL(verifyUrl).searchParams.get('token');
const verified = await json('GET', `/v1/signup/verify?token=${encodeURIComponent(token)}`);
if (verified.status !== 200 || !verified.body.test_key) {
  fail(`verify -> ${verified.status} ${JSON.stringify(verified.body)}`);
}
const deductions = await json(
  'POST',
  '/v1/deductions',
  {
    as_of: '2026-01-15',
    province: 'ON',
    pay_period: 52,
    gross_pay: '1000.00',
    federal_claim_code: 1,
    provincial_claim_code: 1,
  },
  { authorization: `Bearer ${verified.body.test_key}` },
);
const net = deductions.body.employee?.net_pay;
if (deductions.status !== 200 || net !== '800.79') {
  fail(`authenticated deductions net_pay=${net} status=${deductions.status}`);
}

const conformance = await fetch(`${ORIGIN}/conformance/`);
if (!conformance.ok) fail(`GET /conformance/ -> ${conformance.status}`);
const conformanceHtml = await conformance.text();
const ids = new Set(
  conformanceHtml.match(/[A-Za-z]+-P\d+-[0-9.]+-F[01]-P[01]/g) ?? [],
);
if (ids.size < 164) {
  fail(`/conformance listed ${ids.size} disagreement ids, need 164`);
}
if (!conformanceHtml.includes('AB-P10-11704.49-F0-P0')) {
  fail('/conformance missing first disagreement id');
}
if (!conformanceHtml.includes('YT-P24-208.33-F0-P0')) {
  fail('/conformance missing last disagreement id');
}

const browser = await chromium.launch();
const page = await browser.newPage();
await page.goto(`${ORIGIN}/calculators/on/`, { waitUntil: 'networkidle' });
await page.locator('[data-testid="as-of"]').fill('2026-01-15');
await page.locator('[data-testid="pay-period"]').selectOption('52');
await page.locator('[data-testid="federal-claim"]').selectOption('1');
await page.locator('[data-testid="provincial-claim"]').selectOption('1');
await page.locator('[data-testid="gross-pay"]').fill('1000.00');
await page.getByTestId('net-pay').filter({ hasText: /^800\.79$/ }).waitFor({ timeout: 30_000 });
const pageNet = await page.getByTestId('net-pay').innerText();
await browser.close();
if (pageNet !== '800.79') fail(`calculator page net-pay=${pageNet}`);

process.stdout.write(
  `live smoke ok: ${ORIGIN} calculator=800.79 api=800.79 conformance=${ids.size}\n`,
);
