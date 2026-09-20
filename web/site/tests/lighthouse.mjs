/**
 * Lighthouse performance + accessibility floors (spec §13.3 / test 8).
 *
 * Local / pre-push: one run, performance >= 0.99, accessibility >= 0.99.
 * GitHub Actions: three runs, median(performance) >= 0.95, accessibility >= 0.99
 * on every sample. Accessibility does not get a softer CI floor.
 */
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import lighthouse from 'lighthouse';
import { lighthouseMode, median } from './lighthouse-math.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const THRESHOLDS = JSON.parse(
  readFileSync(path.join(ROOT, 'tests/lighthouse-thresholds.json'), 'utf8'),
);
const PORT = 4322;
const DEBUG_PORT = 9222;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const MODE = lighthouseMode();
const RUNS = MODE === 'ci' ? THRESHOLDS.ci_runs : 1;
const PERF_FLOOR =
  MODE === 'ci' ? THRESHOLDS.performance_ci_median : THRESHOLDS.performance;
const CHROME_FLAGS = [
  `--remote-debugging-port=${DEBUG_PORT}`,
  '--no-sandbox',
  '--disable-gpu',
  '--disable-dev-shm-usage',
];

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

async function oneRun(port) {
  const result = await lighthouse(`${ORIGIN}${THRESHOLDS.url}`, {
    port,
    output: 'json',
    onlyCategories: ['performance', 'accessibility'],
    logLevel: 'error',
  });
  const report = result.lhr;
  return {
    performance: report.categories.performance.score,
    accessibility: report.categories.accessibility.score,
    report,
  };
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
    args: CHROME_FLAGS,
  });

  const performances = [];
  const accessibilities = [];
  let lastReport;
  for (let attempt = 1; attempt <= RUNS; attempt++) {
    const sample = await oneRun(DEBUG_PORT);
    performances.push(sample.performance);
    accessibilities.push(sample.accessibility);
    lastReport = sample.report;
    console.log(
      `lighthouse ${MODE} run ${attempt}/${RUNS}: performance=${sample.performance} accessibility=${sample.accessibility}`,
    );
    assert.ok(
      sample.accessibility >= THRESHOLDS.accessibility,
      `accessibility ${sample.accessibility} < ${THRESHOLDS.accessibility} on run ${attempt}`,
    );
  }

  const perfScore = MODE === 'ci' ? median(performances) : performances[0];
  console.log(
    `lighthouse ${MODE}: performance=${perfScore} (floor ${PERF_FLOOR}; samples=[${performances.join(', ')}]) accessibility all >= ${THRESHOLDS.accessibility}`,
  );

  writeFileSync(
    path.join(ROOT, 'lighthouse-report.json'),
    JSON.stringify(
      {
        mode: MODE,
        performances,
        accessibilities,
        performance_asserted: perfScore,
        performance_floor: PERF_FLOOR,
        accessibility_floor: THRESHOLDS.accessibility,
        last: lastReport,
      },
      null,
      2,
    ),
  );

  assert.ok(
    perfScore >= PERF_FLOOR,
    `performance ${perfScore} < ${PERF_FLOOR} (mode=${MODE}, samples=[${performances.join(', ')}])`,
  );
} finally {
  if (browser) {
    await browser.close();
  }
  preview.kill('SIGTERM');
}
