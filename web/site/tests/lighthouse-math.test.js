/**
 * Lighthouse gate math and threshold contract.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { lighthouseMode, median } from './lighthouse-math.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const THRESHOLDS = JSON.parse(
  readFileSync(path.join(ROOT, 'tests/lighthouse-thresholds.json'), 'utf8'),
);

test('median of three runs is the middle score', () => {
  assert.equal(median([0.84, 0.97, 0.96]), 0.96);
  assert.equal(median([0.99, 0.99, 0.99]), 0.99);
  assert.equal(median([0.8, 0.95, 1]), 0.95);
});

test('lighthouseMode defaults: GITHUB_ACTIONS => ci, else local', () => {
  assert.equal(lighthouseMode({}), 'local');
  assert.equal(lighthouseMode({ GITHUB_ACTIONS: 'true' }), 'ci');
  assert.equal(lighthouseMode({ GITHUB_ACTIONS: 'true', TAKEHOME_LIGHTHOUSE_MODE: 'local' }), 'local');
  assert.equal(lighthouseMode({ TAKEHOME_LIGHTHOUSE_MODE: 'ci' }), 'ci');
});

test('thresholds keep a strict local floor and a CI median floor', () => {
  assert.equal(THRESHOLDS.performance, 0.99);
  assert.equal(THRESHOLDS.performance_ci_median, 0.95);
  assert.equal(THRESHOLDS.accessibility, 0.99);
  assert.equal(THRESHOLDS.ci_runs, 3);
  assert.ok(THRESHOLDS.performance_ci_median < THRESHOLDS.performance);
  assert.ok(THRESHOLDS.performance_ci_median >= 0.95);
});
