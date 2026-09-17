/**
 * Lighthouse performance + accessibility floors (spec §13.3 / test 8).
 */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import lighthouse from 'lighthouse';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const THRESHOLDS = JSON.parse(
  readFileSync(path.join(ROOT, 'tests/lighthouse-thresholds.json'), 'utf8'),
);
const PORT = 4322;
const DEBUG_PORT = 9222;
const ORIGIN = `http://127.0.0.1:${PORT}`;

function waitForServer() {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const tick = () => {
      const req = http.get(`${ORIGIN}/`, (res) => {
        res.resume();
        resolve();
      });
      req.on('error', () => {
        if (Date.now() - started > 60_000) {
          reject(new Error('preview server did not start'));
          return;
        }
        setTimeout(tick, 250);
      });
    };
    tick();
  });
}

const preview = spawn(
  path.join(ROOT, 'node_modules/.bin/astro'),
  ['preview', '--host', '127.0.0.1', '--port', String(PORT)],
  { cwd: ROOT, stdio: 'inherit' },
);

let browser;
try {
  await waitForServer();
  browser = await chromium.launch({
    headless: true,
    args: [`--remote-debugging-port=${DEBUG_PORT}`, '--no-sandbox', '--disable-gpu'],
  });
  const result = await lighthouse(`${ORIGIN}${THRESHOLDS.url}`, {
    port: DEBUG_PORT,
    output: 'json',
    onlyCategories: ['performance', 'accessibility'],
    logLevel: 'error',
  });
  const report = result.lhr;
  writeFileSync(
    path.join(ROOT, 'lighthouse-report.json'),
    JSON.stringify(report, null, 2),
  );
  const performance = report.categories.performance.score;
  const accessibility = report.categories.accessibility.score;
  console.log(
    `lighthouse performance=${performance} (floor ${THRESHOLDS.performance}) accessibility=${accessibility} (floor ${THRESHOLDS.accessibility})`,
  );
  assert.ok(
    performance >= THRESHOLDS.performance,
    `performance ${performance} < ${THRESHOLDS.performance}`,
  );
  assert.ok(
    accessibility >= THRESHOLDS.accessibility,
    `accessibility ${accessibility} < ${THRESHOLDS.accessibility}`,
  );
} finally {
  if (browser) {
    await browser.close();
  }
  preview.kill('SIGTERM');
}
