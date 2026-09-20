import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { canonicalize } from '../src/canonicalize.js';
import { sha256, watch } from '../src/watch.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.join(here, '../fixtures');

function load(name) {
  return readFileSync(path.join(fixtures, name), 'utf8');
}

function response(body) {
  return {
    ok: true,
    status: 200,
    arrayBuffer: async () => Buffer.from(body, 'utf8'),
  };
}

test('45. a changed fixture opens an issue whose body contains the diff', async () => {
  const pinHtml = load('index.pin.html');
  const newHtml = load('index.changed.html');
  const pinCsv = load('rates.pin.csv');
  const newCsv = load('rates.changed.csv');
  const pins = {
    documents: [
      {
        id: 't4127-index',
        url: 'https://example.test/t4127.html',
        kind: 'html',
        sha256: sha256(canonicalize('html', pinHtml)),
        snapshot: path.join(fixtures, 'index.pin.html'),
      },
    ],
    csv_bundle: [
      {
        name: 'rates.csv',
        sha256: sha256(canonicalize('csv', pinCsv)),
        snapshot: path.join(fixtures, 'rates.pin.csv'),
      },
    ],
  };
  const bodies = {
    'https://example.test/t4127.html': newHtml,
    'https://example.test/csv/rates.csv': newCsv,
  };
  const issues = [];
  await watch({
    pins,
    fetchImpl: async (url) => {
      const body = bodies[url];
      assert.ok(body, `unexpected fetch ${url}`);
      return response(body);
    },
    openIssue: async (issue) => {
      issues.push(issue);
    },
    readSnapshot: (rel) => readFileSync(rel, 'utf8'),
  });
  assert.ok(issues.length >= 1, 'expected at least one issue');
  const htmlIssue = issues.find((row) => row.id === 't4127-index');
  assert.ok(htmlIssue, `issues: ${issues.map((row) => row.id)}`);
  assert.match(htmlIssue.body, /```diff/);
  assert.match(htmlIssue.diff, /--- pin\/t4127-index/);
  assert.match(htmlIssue.diff, /\+.*0\.0614/);
  assert.match(htmlIssue.diff, /-.*0\.0506/);
  const csvIssue = issues.find((row) => row.id === 'csv:rates.csv');
  assert.ok(csvIssue, 'CSV bundle drift must also open');
  assert.match(csvIssue.diff, /0\.0614/);
  assert.match(csvIssue.diff, /0\.0506/);
});

test('49. pin drift raises a rules-watch alert', async () => {
  const pinHtml = load('index.pin.html');
  const newHtml = load('index.changed.html');
  const pinCsv = load('rates.pin.csv');
  const newCsv = load('rates.changed.csv');
  const pins = {
    documents: [
      {
        id: 't4127-index',
        url: 'https://example.test/t4127.html',
        kind: 'html',
        sha256: sha256(canonicalize('html', pinHtml)),
        snapshot: path.join(fixtures, 'index.pin.html'),
      },
    ],
    csv_bundle: [
      {
        name: 'rates.csv',
        sha256: sha256(canonicalize('csv', pinCsv)),
        snapshot: path.join(fixtures, 'rates.pin.csv'),
      },
    ],
  };
  const bodies = {
    'https://example.test/t4127.html': newHtml,
    'https://example.test/csv/rates.csv': newCsv,
  };
  const issues = [];
  const alerts = [];
  await watch({
    pins,
    fetchImpl: async (url) => {
      const body = bodies[url];
      assert.ok(body, `unexpected fetch ${url}`);
      return response(body);
    },
    openIssue: async (issue) => {
      issues.push(issue);
    },
    onAlert: async (event) => {
      alerts.push(event);
    },
    readSnapshot: (rel) => readFileSync(rel, 'utf8'),
  });
  assert.ok(issues.length >= 1);
  assert.ok(alerts.some((row) => row.type === 'rules_watch_drift'));
  assert.ok(alerts.some((row) => row.id === 't4127-index'));
});

test('45. unchanged fixture does not open an issue', async () => {
  const pinHtml = load('index.pin.html');
  const pinCsv = load('rates.pin.csv');
  const pins = {
    documents: [
      {
        id: 't4127-index',
        url: 'https://example.test/t4127.html',
        kind: 'html',
        sha256: sha256(canonicalize('html', pinHtml)),
        snapshot: path.join(fixtures, 'index.pin.html'),
      },
    ],
    csv_bundle: [
      {
        name: 'rates.csv',
        sha256: sha256(canonicalize('csv', pinCsv)),
        snapshot: path.join(fixtures, 'rates.pin.csv'),
      },
    ],
  };
  const issues = [];
  await watch({
    pins,
    fetchImpl: async (url) => {
      if (url.endsWith('rates.csv')) {
        return response(pinCsv);
      }
      return response(pinHtml);
    },
    openIssue: async (issue) => issues.push(issue),
    readSnapshot: (rel) => readFileSync(rel, 'utf8'),
  });
  assert.deepEqual(issues, []);
});

test('45. Date modified chrome does not count as a change', async () => {
  const pinHtml = load('index.pin.html');
  const retouched = pinHtml
    .replace('2026-01-01', '2026-09-18')
    .replace('2026-01-01', '2026-09-18');
  const digestPin = sha256(canonicalize('html', pinHtml));
  const digestNew = sha256(canonicalize('html', retouched));
  assert.equal(digestPin, digestNew);
});
