import { expect, test } from '@playwright/test';
import { readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { engineCalculate } from './helpers.js';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const ANNUAL_ON_75000 = {
  as_of: '2026-07-01',
  province: 'ON',
  pay_period: 1,
  gross_pay: '75000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
  cpp_months: 12,
};

test('29. embed.js is a small classic script, not a bundle', () => {
  const file = path.join(ROOT, 'public', 'embed.js');
  const src = readFileSync(file, 'utf8');
  const bytes = statSync(file).size;
  expect(bytes, `embed.js ${bytes} bytes`).toBeLessThanOrEqual(4096);
  expect(src).toMatch(/attachShadow/);
});

test('29. shadow root isolates host and widget styles', async ({ page }) => {
  await page.goto('/embed-isolate.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  const isolation = await widget.evaluate((el) => {
    const root = el.shadowRoot;
    const net = root && root.querySelector('[data-testid="takehome-embed-net"]');
    const hostH1 = document.querySelector('h1');
    const hostP = document.querySelector('body > p');
    return {
      shadow: Boolean(root),
      netText: net ? net.textContent : '',
      netColor: net ? getComputedStyle(net).color : '',
      netSize: net ? getComputedStyle(net).fontSize : '',
      h1Color: hostH1 ? getComputedStyle(hostH1).color : '',
      h1Size: hostH1 ? getComputedStyle(hostH1).fontSize : '',
      hostPColor: hostP ? getComputedStyle(hostP).color : '',
    };
  });
  expect(isolation.shadow).toBe(true);
  expect(isolation.netText).toMatch(/^\d/);
  expect(isolation.netColor).not.toBe('rgb(255, 0, 0)');
  expect(isolation.netSize).not.toBe('80px');
  expect(isolation.h1Color).toBe('rgb(255, 0, 0)');
  expect(isolation.h1Size).toBe('80px');
  expect(isolation.hostPColor).toBe('rgb(255, 0, 0)');
});

test('29. spec one-liner renders ON $75000 take-home matching the engine', async ({
  page,
}) => {
  const expected = engineCalculate(ANNUAL_ON_75000);
  await page.goto('/embed-host.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(widget.getByTestId('takehome-embed-net')).toHaveText(
    expected.employee.net_pay,
  );
});

test('29. weekly M1 vector through the widget is still 800.79', async ({
  page,
}) => {
  await page.goto('/embed-weekly.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(widget.getByTestId('takehome-embed-net')).toHaveText('800.79');
});

test('29. after load, calculation does not hit the network or set a cookie', async ({
  page,
  context,
}) => {
  const leaked = [];
  page.on('request', (req) => {
    const url = req.url();
    const body = req.postData() ?? '';
    if (url.includes('75000') || body.includes('75000')) {
      leaked.push(url);
    }
    if (/google-analytics|doubleclick|segment|mixpanel|plausible/i.test(url)) {
      leaked.push(url);
    }
  });
  await page.goto('/embed-host.html');
  await expect(page.locator('[data-testid="takehome-embed"]')).toHaveAttribute(
    'data-ready',
    'true',
    { timeout: 30_000 },
  );
  const after = [];
  page.on('request', (req) => after.push(req.url()));
  await page.waitForTimeout(250);
  expect(after, after.join(', ')).toEqual([]);
  expect(leaked, leaked.join(', ')).toEqual([]);
  expect(await context.cookies()).toEqual([]);
});

test('29. Quebec is refused, not shown as federal-only', async ({ page }) => {
  await page.goto('/embed-qc.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(widget).toContainText(/not supported/i);
  await expect(widget.getByTestId('takehome-embed-net')).toHaveCount(0);
});

test('30. demo page embeds the widget and pitches job boards', async ({
  page,
}) => {
  await page.goto('/embed/');
  await expect(page.locator('h1')).toContainText(/embed/i);
  await expect(page.locator('main')).toContainText(/job boards/i);
  await expect(page.locator('main')).toContainText(
    'src="https://takehome.gautamkhosla.com/embed/v0.1.0/embed.js"',
  );
  await expect(page.locator('main')).toContainText('integrity="sha384-');
  await expect(page.locator('main')).toContainText('crossorigin="anonymous"');
  await expect(page.locator('main')).not.toContainText(/contact sales/i);
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  const expected = engineCalculate(ANNUAL_ON_75000);
  await expect(widget.getByTestId('takehome-embed-net')).toHaveText(
    expected.employee.net_pay,
  );
});
