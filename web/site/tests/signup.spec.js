import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { blockFor, parseHeadersFile } from './parse-headers.js';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const HEADERS = parseHeadersFile(
  readFileSync(path.join(ROOT, 'public/_headers'), 'utf8'),
);

test('signup form POSTs /v1/signup under production CSP', async ({ page }) => {
  const cspHits = [];
  page.on('console', (msg) => {
    const text = msg.text();
    if (/content security policy|csp/i.test(text)) cspHits.push(text);
  });
  page.on('pageerror', (err) => {
    cspHits.push(String(err));
  });

  let signupRequest = null;
  await page.route('**/v1/signup', async (route) => {
    signupRequest = route.request();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        message:
          'Check your email. After you verify, you receive a test key and a live key and can make the first call.',
      }),
    });
  });

  const star = blockFor(HEADERS, '/*');
  await page.route('**/*', async (route) => {
    if (route.request().url().includes('/v1/signup')) {
      await route.fallback();
      return;
    }
    const response = await route.fetch();
    const headers = { ...response.headers(), ...star.headers };
    await route.fulfill({ response, headers });
  });

  await page.goto('/signup/');
  await page.locator('input[name="email"]').fill('csp-signup@example.com');
  await page.locator('button[type="submit"]').click();

  await expect(page.locator('#signup-status')).toContainText(/check your email/i, {
    timeout: 10_000,
  });
  expect(signupRequest, 'POST /v1/signup must fire').toBeTruthy();
  expect(signupRequest.method()).toBe('POST');
  expect(await signupRequest.postDataJSON()).toEqual({
    email: 'csp-signup@example.com',
  });
  expect(page.url()).not.toMatch(/[?&]email=/);
  expect(cspHits, cspHits.join('\n')).toEqual([]);
});
