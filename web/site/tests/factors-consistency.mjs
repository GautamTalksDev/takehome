/**
 * Three-way factor-set equality: page rows, engine FACTOR_KEYS, data/factors.json.
 * A passing check must fail when any set is broken on purpose.
 */
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const REPO = path.resolve(ROOT, '../..');
const DIST = path.join(ROOT, 'dist');

export function catalogSymbols(raw) {
  return JSON.parse(raw).factors.map((row) => row.symbol);
}

export function factorKeysFromResponseRs(src) {
  const match = src.match(/FACTOR_KEYS:.*?=\s*&\[([\s\S]*?)\]/);
  assert.ok(match, 'FACTOR_KEYS block missing from response.rs');
  return [...match[1].matchAll(/"([A-Z0-9]+)"/g)].map((row) => row[1]);
}

export function pageFactorSymbols(html) {
  return [...html.matchAll(/data-factor="([^"]+)"/g)].map((row) => row[1]);
}

export function setDiff(left, right) {
  const l = new Set(left);
  const r = new Set(right);
  return {
    onlyLeft: [...l].filter((k) => !r.has(k)).sort(),
    onlyRight: [...r].filter((k) => !l.has(k)).sort(),
  };
}

export function assertSameFactorSets(page, breakdown, catalog) {
  const pageVbreakdown = setDiff(page, breakdown);
  const pageVcatalog = setDiff(page, catalog);
  const breakdownVcatalog = setDiff(breakdown, catalog);
  const empty =
    pageVbreakdown.onlyLeft.length === 0 &&
    pageVbreakdown.onlyRight.length === 0 &&
    pageVcatalog.onlyLeft.length === 0 &&
    pageVcatalog.onlyRight.length === 0 &&
    breakdownVcatalog.onlyLeft.length === 0 &&
    breakdownVcatalog.onlyRight.length === 0;
  assert.ok(
    empty,
    `factor sets drifted:
page vs breakdown ${JSON.stringify(pageVbreakdown)}
page vs catalog ${JSON.stringify(pageVcatalog)}
breakdown vs catalog ${JSON.stringify(breakdownVcatalog)}`,
  );
  assert.equal(page.length, catalog.length);
  assert.equal(breakdown.length, catalog.length);
}

const catalog = catalogSymbols(
  readFileSync(path.join(REPO, 'data/factors.json'), 'utf8'),
);
const breakdown = factorKeysFromResponseRs(
  readFileSync(path.join(REPO, 'crates/takehome-core/src/response.rs'), 'utf8'),
);

assert.ok(existsSync(DIST), 'dist missing; run npm run build first');

function walk(dir, acc = []) {
  for (const name of readdirSync(dir)) {
    const full = path.join(dir, name);
    if (statSync(full).isDirectory()) {
      walk(full, acc);
    } else {
      acc.push(full);
    }
  }
  return acc;
}

const calculatorPages = walk(DIST).filter((file) =>
  file.endsWith(`${path.sep}calculators${path.sep}on${path.sep}index.html`),
);
assert.equal(calculatorPages.length, 1, 'expected one ON calculator page');
const page = pageFactorSymbols(readFileSync(calculatorPages[0], 'utf8'));

assertSameFactorSets(page, breakdown, catalog);

const brokenCatalog = [...catalog, 'ZZZ_NOT_A_FACTOR'];
assert.throws(
  () => assertSameFactorSets(page, breakdown, brokenCatalog),
  /factor sets drifted/,
);
const brokenPage = page.filter((k) => k !== 'QPIP');
assert.throws(
  () => assertSameFactorSets(brokenPage, breakdown, catalog),
  /factor sets drifted/,
);

const spec = readFileSync(path.join(ROOT, 'tests/calculator.spec.js'), 'utf8');
assert.match(spec, /toHaveCount\(CATALOG\.length\)/);
assert.match(spec, /\[\.\.\.rendered\]\.sort\(\)/);
assert.match(spec, /ZZZ_NOT_A_FACTOR/);
assert.doesNotMatch(
  spec,
  /factor-row[\s\S]{0,120}toHaveCount\(\d+\)/,
  'Playwright must not hard-code a factor count; read data/factors.json',
);

console.log(
  `factors-consistency ok: ${catalog.length} keys; page=breakdown=catalog; mutation fails`,
);
