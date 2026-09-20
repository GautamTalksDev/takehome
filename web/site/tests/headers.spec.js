import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  fillKnownOntarioWeekly,
  waitForEngine,
} from './helpers.js';
import { blockFor, parseHeadersFile } from './parse-headers.js';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const HEADERS_TEXT = readFileSync(path.join(ROOT, 'public/_headers'), 'utf8');
const HEADERS = parseHeadersFile(HEADERS_TEXT);

const REQUIRED = {
  'Content-Security-Policy':
    "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'; object-src 'none'",
  'Strict-Transport-Security':
    'max-age=31536000; includeSubDomains; preload',
  'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'strict-origin-when-cross-origin',
  'Permissions-Policy':
    'geolocation=(), camera=(), microphone=(), payment=(), interest-cohort=()',
  'Cross-Origin-Opener-Policy': 'same-origin',
};

async function applySiteHeaders(page, extra = {}) {
  const star = blockFor(HEADERS, '/*');
  await page.route('**/*', async (route) => {
    const response = await route.fetch();
    const headers = { ...response.headers(), ...star.headers, ...extra };
    await route.fulfill({ response, headers });
  });
}

test('14. site _headers pin the A02 security set and wasm-unsafe-eval only', () => {
  const star = blockFor(HEADERS, '/*');
  for (const [name, value] of Object.entries(REQUIRED)) {
    expect(star.headers[name], name).toBe(value);
  }
  const csp = star.headers['Content-Security-Policy'];
  expect(csp).toContain("'wasm-unsafe-eval'");
  expect(csp).not.toMatch(/(?<!wasm-)'unsafe-eval'/);
  expect(csp).not.toContain("'unsafe-inline'");
});

test('14. calculator still runs with those headers enforced', async ({
  page,
}) => {
  const cspHits = [];
  page.on('console', (msg) => {
    if (/content security policy/i.test(msg.text())) {
      cspHits.push(msg.text());
    }
  });
  page.on('pageerror', (err) => {
    cspHits.push(String(err));
  });
  await applySiteHeaders(page);
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  await expect(page.getByTestId('net-pay')).toHaveText('800.79');
  expect(cspHits, cspHits.join('\n')).toEqual([]);
});

test('15. embed.js has cross-origin headers and written CSP reasoning', () => {
  const embed = blockFor(HEADERS, '/embed.js');
  expect(embed.headers['Access-Control-Allow-Origin']).toBe('*');
  expect(embed.headers['Cross-Origin-Resource-Policy']).toBe('cross-origin');
  const pinned = blockFor(HEADERS, '/embed/v0.1.0/embed.js');
  expect(pinned.headers['Access-Control-Allow-Origin']).toBe('*');
  expect(pinned.headers['Cross-Origin-Resource-Policy']).toBe('cross-origin');
  expect(pinned.headers['Cache-Control']).toBe(
    'public, max-age=31536000, immutable',
  );
  expect(HEADERS_TEXT).toMatch(/unsafe-inline/);
  expect(HEADERS_TEXT).toMatch(/unsafe-eval/);
  expect(HEADERS_TEXT).toMatch(/must not|do not need|not require/i);
  const engine = blockFor(HEADERS, '/engine/*');
  expect(engine.headers['Access-Control-Allow-Origin']).toBe('*');
  expect(engine.headers['Cross-Origin-Resource-Policy']).toBe('cross-origin');
});

test('15. embed.js does not need the host to add unsafe-inline or unsafe-eval', async ({
  page,
}) => {
  const src = readFileSync(path.join(ROOT, 'public', 'embed.js'), 'utf8');
  expect(src).not.toMatch(/\beval\s*\(/);
  expect(src).not.toMatch(/new Function\s*\(/);
  expect(src).not.toMatch(/innerHTML\s*=/);

  const cspHits = [];
  page.on('console', (msg) => {
    if (/content security policy/i.test(msg.text())) {
      cspHits.push(msg.text());
    }
  });
  await applySiteHeaders(page);
  await page.goto('/embed-host.html');
  const widget = page.locator('[data-testid="takehome-embed"]');
  await expect(widget).toHaveAttribute('data-ready', 'true', { timeout: 30_000 });
  await expect(widget.getByTestId('takehome-embed-net')).toHaveText(/^\d/);
  expect(cspHits.join('\n')).not.toMatch(/'unsafe-inline'/);
  expect(cspHits.join('\n')).not.toMatch(/(?<!wasm-)'unsafe-eval'/);
});
