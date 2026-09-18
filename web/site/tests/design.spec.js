import { expect, test } from '@playwright/test';
import { fillKnownOntarioWeekly, waitForEngine } from './helpers.js';

test('header is wordmark plus quiet nav and one accent CTA', async ({ page }) => {
  await page.goto('/');
  const header = page.locator('header');
  const wordmark = header.locator('.wordmark svg');
  await expect(wordmark).toBeVisible();
  const svg = await wordmark.evaluate((el) => el.outerHTML);
  expect(Buffer.byteLength(svg, 'utf8')).toBeLessThan(2048);

  await expect(header.getByRole('link', { name: 'Docs', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Pricing', exact: true })).toBeVisible();
  await expect(
    header.getByRole('link', { name: 'Conformance', exact: true }),
  ).toBeVisible();
  await expect(header.getByRole('link', { name: 'Get API keys' })).toBeVisible();
  await expect(header.locator('summary')).toHaveText('Calculators');

  await expect(header.getByRole('link', { name: 'ON', exact: true })).toHaveCount(0);
  await header.locator('summary').click();
  await expect(header.getByRole('link', { name: 'Ontario' })).toBeVisible();
});

test('result panel splits employee deductions from collapsed employer cost', async ({
  page,
}) => {
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);

  const net = page.getByTestId('net-pay');
  await expect(net).not.toHaveText('');
  await expect(net).toHaveCSS('font-variant-numeric', /tabular-nums/);

  await expect(page.getByRole('heading', { name: 'Your deductions' })).toBeVisible();
  const employer = page.getByTestId('employer-block');
  await expect(employer).toBeVisible();
  await expect(employer).not.toHaveAttribute('open');
  await expect(page.getByTestId('employer-cpp')).not.toBeVisible();
  await employer.locator('summary').click();
  await expect(page.getByTestId('employer-cpp')).toBeVisible();
  await expect(employer).toContainText('not withheld');
  await expect(page.getByTestId('rule-set')).toBeVisible();
});

test('citations name the factor, what it is, and the table, with a deep link', async ({
  page,
}) => {
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  const links = page.getByTestId('citations').locator('a[href^="http"]');
  await expect(links).not.toHaveCount(0);
  const texts = await links.allTextContents();
  expect(new Set(texts).size).toBe(texts.length);
  expect(texts.some((row) => /123rd Edition/i.test(row))).toBe(false);

  const r = page.getByTestId('citations').locator('a', { hasText: 'R —' });
  await expect(r).toContainText('federal tax rate');
  await expect(r).toContainText('Table 8.1');
  await expect(r).toHaveAttribute('href', /#toc\d+/);
});

test('calculator lead is plain language; T4127 depth sits below the answer', async ({
  page,
}) => {
  await page.goto('/take-home-pay/on/');
  await expect(page.locator('.lede')).not.toContainText('T4127');
  await expect(page.locator('.lede')).not.toContainText('surtax');
  await expect(page.locator('.lede')).not.toContainText(
    'The figure on the right is T4127 net pay',
  );
  await expect(page.locator('.prose')).toContainText('surtax');
  await expect(page.locator('.prose')).toContainText('Health Premium');

  await page.goto('/take-home-pay/ab/');
  await expect(page.locator('.lede')).not.toContainText('K5P');
  await expect(page.locator('.prose')).toContainText('K5P');
  await expect(page.locator('.prose h2').first()).toContainText('not flat');

  await page.goto('/take-home-pay/bc/');
  await expect(page.locator('.lede')).not.toContainText('T4127');
  await expect(page.locator('.prose')).toContainText('tax reduction');
  await expect(page.locator('.prose')).toContainText('prorat');
});

test('pricing is equal cards with a CTA and no sales queue', async ({ page }) => {
  await page.goto('/pricing/');
  const cards = page.locator('.plan');
  await expect(cards).toHaveCount(5);
  for (const card of await cards.all()) {
    await expect(card.locator('.plan-name')).not.toHaveText('');
    await expect(card.locator('.plan-price')).not.toHaveText('');
    await expect(card.locator('.plan-for')).not.toHaveText('');
    await expect(card.getByRole('link', { name: 'Get API keys' })).toBeVisible();
  }
  await expect(page.locator('main')).not.toContainText('contact sales');
});

test('docs puts a highlighted first call above the fold with copy and a left nav', async ({
  page,
}) => {
  await page.goto('/docs/');
  const h1Box = await page.locator('h1').boundingBox();
  const example = page.locator('#first-call');
  const exampleBox = await example.boundingBox();
  expect(h1Box).toBeTruthy();
  expect(exampleBox).toBeTruthy();
  expect(exampleBox.y).toBeLessThan(700);

  await expect(page.getByRole('navigation', { name: 'Docs' })).toBeVisible();
  await expect(example.locator('.shiki')).toBeVisible();
  await expect(page.locator('main')).toContainText('800.79');
  await expect(page.locator('main')).toContainText('authorization: Bearer');

  const copy = example.locator('button.copy');
  await expect(copy).toBeVisible();
  await copy.click();
  await expect(copy).toHaveText('Copied');
});
