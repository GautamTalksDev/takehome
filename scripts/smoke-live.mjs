#!/usr/bin/env node
/**
 * Live-origin smoke. M3 surfaces plus M4: batch, year projection, bonus TB,
 * BC Option 1/Option 2 split, and the 2027 preview page with the banner
 * above the number.
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

const ON_WEEKLY = {
  as_of: '2026-01-15',
  province: 'ON',
  pay_period: 52,
  gross_pay: '1000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
};

const home = await fetch(`${ORIGIN}/`);
if (!home.ok) fail(`GET / -> ${home.status}`);
const homeHtml = await home.text();
if (!homeHtml.includes('Takehome')) fail('home page missing Takehome');

const signup = await json('POST', '/v1/signup', {
  email: `smoke-${Date.now()}@example.com`,
});
if (signup.status !== 200) fail(`/v1/signup -> ${signup.status} ${JSON.stringify(signup.body)}`);
if (signup.body.verify_url) {
  fail('live origin echoed verify_url — ECHO_VERIFY_URL must stay off production');
}
const serializedSignup = JSON.stringify(signup.body);
if (/https?:\/\//i.test(serializedSignup) || /"[^"]*token[^"]*"\s*:/.test(serializedSignup)) {
  fail(`signup body leaked a URL or token field: ${serializedSignup}`);
}
const smokeKey = process.env.SMOKE_API_KEY;
if (!smokeKey) {
  fail('set SMOKE_API_KEY; live signup no longer echoes the verify URL');
}
const auth = { authorization: `Bearer ${smokeKey}` };

const deductions = await json('POST', '/v1/deductions', ON_WEEKLY, auth);
const net = deductions.body.employee?.net_pay;
if (deductions.status !== 200 || net !== '800.79') {
  fail(`authenticated deductions net_pay=${net} status=${deductions.status}`);
}

const batchBody = {
  requests: Array.from({ length: 1000 }, (_, i) =>
    i % 2 === 0
      ? { ...ON_WEEKLY }
      : { ...ON_WEEKLY, as_of: '2026-07-01' },
  ),
};
const batch = await json('POST', '/v1/deductions/batch', batchBody, auth);
if (batch.status !== 200) {
  fail(`/v1/deductions/batch -> ${batch.status} ${JSON.stringify(batch.body).slice(0, 300)}`);
}
if (!Array.isArray(batch.body.results) || batch.body.results.length !== 1000) {
  fail(`batch results length=${batch.body.results?.length}, need 1000`);
}
if (batch.body.results[0]?.response?.employee?.net_pay !== '800.79') {
  fail(`batch[0] net_pay=${batch.body.results[0]?.response?.employee?.net_pay}`);
}
if (batch.body.results[0]?.response?.rule_set_version !== '2026-01-01') {
  fail(`batch[0] rule_set_version=${batch.body.results[0]?.response?.rule_set_version}`);
}
if (batch.body.results[1]?.response?.rule_set_version !== '2026-07-01') {
  fail(`batch[1] rule_set_version=${batch.body.results[1]?.response?.rule_set_version}`);
}

const year = await json(
  'POST',
  '/v1/deductions/year',
  {
    as_of: '2026-01-01',
    province: 'ON',
    pay_period: 26,
    gross_pay: '4000.00',
    federal_claim_code: 1,
    provincial_claim_code: 1,
  },
  auth,
);
if (year.status !== 200) {
  fail(`/v1/deductions/year -> ${year.status} ${JSON.stringify(year.body).slice(0, 300)}`);
}
if (year.body.totals?.cpp !== '4230.45') {
  fail(`year totals.cpp=${year.body.totals?.cpp}, need 4230.45`);
}

const bonus = await json(
  'POST',
  '/v1/deductions',
  {
    as_of: '2026-01-01',
    province: 'ON',
    pay_period: 52,
    gross_pay: '1000.00',
    bonus: '2500.00',
    ytd_bonus: '1500.00',
    f5b_ytd: '14.60',
    pay_periods_elapsed: 29,
    federal_claim_code: 1,
    provincial_claim_code: 1,
  },
  auth,
);
const tb = bonus.body.breakdown?.TB;
// Combined federal+provincial TB on this CRA worked example. The published
// federal-only slice is still 323.54 (see takehome-core lib.rs); the API
// field TB is the full period withholding difference.
if (bonus.status !== 200 || tb !== '503.72') {
  fail(`CRA bonus example TB=${tb} status=${bonus.status}`);
}

const bcBase = {
  as_of: '2026-08-01',
  province: 'BC',
  pay_period: 52,
  gross_pay: '800.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
};
const bc1 = await json(
  'POST',
  '/v1/deductions',
  { ...bcBase, calculation_option: 'option1' },
  auth,
);
const bc2 = await json(
  'POST',
  '/v1/deductions',
  { ...bcBase, calculation_option: 'option2' },
  auth,
);
if (bc1.status !== 200 || bc2.status !== 200) {
  fail(`BC Option split status option1=${bc1.status} option2=${bc2.status}`);
}
if (bc1.body.breakdown?.V !== '0.0614') {
  fail(`BC Option 1 V=${bc1.body.breakdown?.V}, need 0.0614`);
}
if (bc2.body.breakdown?.V !== '0.0560') {
  fail(`BC Option 2 V=${bc2.body.breakdown?.V}, need 0.0560`);
}
if (bc1.body.prorated_rules_applied !== true) {
  fail('BC Option 1 must set prorated_rules_applied');
}
if (bc2.body.prorated_rules_applied !== false) {
  fail('BC Option 2 must leave prorated_rules_applied unset');
}

const preview = await json(
  'POST',
  '/v1/deductions',
  { ...ON_WEEKLY, as_of: '2027-01-15' },
  auth,
);
if (preview.status !== 200) {
  fail(`/v1/deductions 2027 -> ${preview.status}`);
}
if (preview.body.rule_set_version !== '2027-01-01') {
  fail(`2027 rule_set_version=${preview.body.rule_set_version}`);
}
const proposed = (preview.body.warnings ?? []).some(
  (warning) => warning.code === 'RULE_SET_PROPOSED',
);
if (!proposed) {
  fail('2027 response missing RULE_SET_PROPOSED warning');
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
if (pageNet !== '800.79') {
  await browser.close();
  fail(`calculator page net-pay=${pageNet}`);
}

await page.goto(`${ORIGIN}/cpp-2027-rate-change/`, { waitUntil: 'networkidle' });
const banner = page.getByTestId('preview-banner');
await banner.waitFor({ state: 'visible', timeout: 30_000 });
const bannerText = await banner.innerText();
if (!/not yet enacted/i.test(bannerText) || !bannerText.includes('2026-04-28')) {
  await browser.close();
  fail(`cpp-2027 preview-banner text=${JSON.stringify(bannerText)}`);
}
const warning = page.getByTestId('proposed-warning');
await warning.waitFor({ state: 'visible', timeout: 30_000 });
const netEl = page.getByTestId('net-pay');
await netEl.waitFor({ state: 'visible', timeout: 30_000 });
const bannerBox = await banner.boundingBox();
const netBox = await netEl.boundingBox();
await browser.close();
if (!bannerBox || !netBox) fail('cpp-2027 missing banner or figure box');
if (!(bannerBox.y < netBox.y)) {
  fail(
    `cpp-2027 preview banner is not above the figure (banner.y=${bannerBox.y} net.y=${netBox.y})`,
  );
}

process.stdout.write(
  `live smoke ok: ${ORIGIN} calculator=800.79 api=800.79 batch=1000 year_cpp=4230.45 TB=503.72 bc_v=0.0614/0.0560 cpp2027_banner_above=yes conformance=${ids.size}\n`,
);
