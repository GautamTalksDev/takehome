import { expect, test } from '@playwright/test';
import {
  calculatorPaths,
  htmlPaths,
} from '../src/lib/catalog.js';
import {
  fillKnownOntarioWeekly,
  waitForEngine,
} from './helpers.js';

test('catalog is 205 HTML pages (spec §13.2 / test 10, plus docs, embed, pricing, signup)', () => {
  expect(htmlPaths()).toHaveLength(205);
});

test('every catalog page renders (test 10)', async ({ request }) => {
  test.setTimeout(120_000);
  const failures = [];
  for (const path of htmlPaths()) {
    const res = await request.get(path);
    if (res.status() !== 200) {
      failures.push(`${path} → ${res.status()}`);
    }
  }
  expect(failures, failures.join('\n')).toEqual([]);
});

test('every calculator computes (test 10)', async ({ page }) => {
  test.setTimeout(480_000);
  const failures = [];
  for (const path of calculatorPaths()) {
    await page.goto(path);
    try {
      await waitForEngine(page);
      const net = await page.getByTestId('net-pay').textContent();
      if (!net || !/^\d+\.\d{2}$/.test(net.trim())) {
        failures.push(`${path} net-pay=${JSON.stringify(net)}`);
      }
    } catch (err) {
      failures.push(`${path} ${err.message}`);
    }
  }
  expect(failures, failures.join('\n')).toEqual([]);
});

test('cpp 2027 page answers the 2027 rate query above the fold', async ({
  page,
}) => {
  await page.goto('/cpp-2027-rate-change/');
  const title = await page.locator('h1').textContent();
  expect(title).toMatch(/CPP/i);
  expect(title).toMatch(/2027/);
  await expect(page.locator('main')).toContainText('YMPE');
  await expect(page.locator('main')).toContainText('5.95');
  await expect(page.locator('main')).toContainText('YAMPE');
});

test('conformance page keeps the 164 disagreements', async ({ page }) => {
  await page.goto('/conformance/');
  await expect(page.locator('h1')).toContainText('Conformance');
  await expect(page.locator('main')).toContainText('AB-P10-11704.49-F0-P0');
  await expect(page.locator('main')).toContainText('YT-P24-208.33-F0-P0');
  await expect(page.locator('main')).toContainText('8838/9002');
});

test('changes page exposes an RSS feed', async ({ page, request }) => {
  await page.goto('/changes/');
  await expect(
    page.getByRole('article').getByRole('link', { name: 'RSS', exact: true }),
  ).toBeVisible();
  const rss = await request.get('/changes.xml');
  expect(rss.status()).toBe(200);
  const body = await rss.text();
  expect(body).toContain('<rss');
  expect(body).toContain('0.3.0-m2');
});

test('ontario take-home page is not a province-name swap of alberta', async ({
  page,
}) => {
  await page.goto('/take-home-pay/on/');
  await expect(page.locator('.lede')).not.toContainText('K5P');
  await expect(page.locator('.prose')).toContainText('surtax');
  await expect(page.locator('.prose')).toContainText('Health Premium');
  await page.goto('/take-home-pay/ab/');
  await expect(page.locator('.prose')).toContainText('K5P');
  await expect(page.locator('.lede')).not.toContainText('Health Premium');
  await expect(page.locator('.prose h2').first()).toContainText('not flat');
  await page.goto('/take-home-pay/bc/');
  await expect(page.locator('.lede')).not.toContainText('T4127');
  await expect(page.locator('.prose')).toContainText('tax reduction');
  await expect(page.locator('.prose')).toContainText('prorat');
});

test('pricing publishes CAD tiers and signup has no sales call', async ({
  page,
}) => {
  await page.goto('/pricing/');
  await expect(page.locator('h1')).toContainText('Pricing');
  await expect(page.locator('main')).toContainText('$49 CAD');
  await expect(page.locator('main')).toContainText('$149 CAD');
  await expect(page.locator('main')).toContainText('$399 CAD');
  await expect(page.locator('main')).not.toContainText('contact sales');
  await page.goto('/signup/');
  await expect(page.locator('h1')).toContainText('Sign up');
  await expect(page.locator('main')).toContainText('No sales call');
  await expect(page.locator('main')).not.toContainText('book a demo');
});

test('26. five-minute path: Docs in the header, first call, get a key', async ({
  page,
}) => {
  await page.goto('/');
  const docs = page.locator('header').getByRole('link', { name: 'Docs', exact: true });
  await expect(docs).toBeVisible();
  await docs.click();
  await expect(page.locator('h1')).toContainText('API docs');
  await expect(page.locator('main')).toContainText('800.79');
  await expect(page.locator('main')).toContainText('authorization: Bearer');
  await expect(page.locator('#first-call').getByRole('link', { name: 'Get a key' })).toBeVisible();
});

test('existing ontario weekly engine match still holds on a generated page', async ({
  page,
}) => {
  await page.goto('/weekly-pay/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  await expect(page.getByTestId('net-pay')).not.toHaveText('');
});
