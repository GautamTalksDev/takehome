import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { fillKnownOntarioWeekly, waitForEngine } from './helpers.js';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const EMBED = readFileSync(path.join(ROOT, 'public', 'embed.js'), 'utf8');
const CALC = readFileSync(path.join(ROOT, 'public', 'engine', 'calculator.js'), 'utf8');

const XSS_GROSS = '"><img src=x onerror=alert(1)>';

test('33. embed allowlists province and gross and never writes innerHTML', () => {
  expect(EMBED).not.toMatch(/innerHTML/);
  expect(EMBED).not.toMatch(/insertAdjacentHTML/);
  expect(EMBED).not.toMatch(/document\.write/);
  expect(EMBED).toMatch(/textContent/);
  expect(EMBED).toMatch(/AB:1/);
  expect(EMBED).toMatch(/OutsideCanada:1/);
  expect(EMBED).toMatch(/function province\b|P\[/);
  expect(EMBED).toMatch(/\\d\+\(\\\.\\d\{1,2\}\)\?/);
});

test('33. data-gross XSS payload does not execute or land in HTML', async ({
  page,
}) => {
  const alerts = [];
  page.on('dialog', async (dialog) => {
    alerts.push(dialog.message());
    await dialog.dismiss();
  });
  await page.goto('/embed-xss.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  expect(alerts).toEqual([]);
  const shadow = await widget.evaluate((el) =>
    el.shadowRoot ? el.shadowRoot.innerHTML : el.innerHTML,
  );
  expect(shadow).not.toMatch(/<img/i);
  expect(shadow).not.toMatch(/onerror/i);
  expect(shadow).not.toContain(XSS_GROSS);
  await expect(widget.getByTestId('takehome-embed-net')).toHaveCount(0);
  await expect(widget).toContainText(/data-province and data-gross/i);
});

test('34. calculator output path uses textContent, not innerHTML', async ({
  page,
}) => {
  expect(CALC).not.toMatch(/innerHTML/);
  expect(CALC).not.toMatch(/insertAdjacentHTML/);
  expect(CALC).not.toMatch(/document\.write/);
  expect(CALC).toMatch(/node\.textContent\s*=/);
  expect(CALC).toMatch(/function text\(/);

  await page.goto('/weekly-pay/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  const net = page.getByTestId('net-pay');
  await expect(net).not.toHaveText('');
  const html = await net.evaluate((el) => el.innerHTML);
  const text = await net.evaluate((el) => el.textContent);
  expect(html).toBe(text);
  expect(html).not.toMatch(/[<>]/);

  await page.getByTestId('gross-pay').fill(XSS_GROSS);
  await page.waitForTimeout(200);
  const after = await net.evaluate((el) => ({
    html: el.innerHTML,
    imgs: el.querySelectorAll('img').length,
  }));
  expect(after.imgs).toBe(0);
  expect(after.html).not.toMatch(/<img/i);
  expect(after.html).not.toMatch(/onerror/i);
});
