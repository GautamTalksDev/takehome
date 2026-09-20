import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  engineCalculate,
  fillKnownOntarioWeekly,
  M1_ON_WEEKLY_1000,
  waitForEngine,
} from './helpers.js';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const CATALOG = JSON.parse(
  readFileSync(path.join(REPO, 'data/factors.json'), 'utf8'),
).factors.map((row) => row.symbol);

test('typed salary matches the native engine (spec §13.3 / test 6)', async ({
  page,
}) => {
  const expected = engineCalculate(M1_ON_WEEKLY_1000);
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  await expect(page.getByTestId('net-pay')).toHaveText(expected.employee.net_pay);
  await expect(page.getByTestId('federal-tax')).toHaveText(
    expected.employee.federal_tax,
  );
  await expect(page.getByTestId('provincial-tax')).toHaveText(
    expected.employee.provincial_tax,
  );
  await expect(page.getByTestId('cpp')).toHaveText(expected.employee.cpp);
  await expect(page.getByTestId('ei')).toHaveText(expected.employee.ei);
  await expect(page.getByTestId('rule-set')).toContainText(expected.rule_set_version);
  await expect(page.getByTestId('rule-set')).toContainText('2026-01-01');
  await expect(page.getByRole('button', { name: /submit/i })).toHaveCount(0);
  await expect(page.locator('dialog')).toHaveCount(0);
  await expect(page.getByTestId('privacy')).toContainText(
    'No salary figure leaves the browser',
  );
});

test('show the working lists every T4127 factor', async ({ page }) => {
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  await page.getByTestId('working').locator('summary').click();
  const rows = page.getByTestId('factor-row');
  await expect(rows).toHaveCount(CATALOG.length);
  const rendered = await rows.evaluateAll((nodes) =>
    nodes.map((node) => node.getAttribute('data-factor')),
  );
  // Set equality against data/factors.json (order preserved from the catalog).
  expect([...rendered].sort()).toEqual([...CATALOG].sort());
  expect(rendered).toEqual(CATALOG);
  // Mutation: an extra or missing symbol must fail the same equality.
  expect([...rendered, 'ZZZ_NOT_A_FACTOR'].sort()).not.toEqual(
    [...CATALOG].sort(),
  );
  expect(rendered.filter((k) => k !== 'QPIP').sort()).not.toEqual(
    [...CATALOG].sort(),
  );
  await expect(page.getByTestId('factor-A')).toBeVisible();
  await expect(page.getByTestId('factor-T')).toBeVisible();
  await expect(page.getByTestId('factor-QPIP')).toBeVisible();
  const citations = page.getByTestId('citations').locator('a[href^="http"]');
  await expect(citations).not.toHaveCount(0);
  await expect(citations.first()).toContainText(' — ');
  await expect(citations.first()).toHaveAttribute('href', /#toc\d+/);
});

test('quebec page is a typed refusal, not a federal-only result', async ({
  page,
}) => {
  await page.goto('/calculators/qc/');
  await expect(page.getByTestId('unsupported')).toBeVisible();
  await expect(page.getByTestId('net-pay')).toHaveCount(0);
});
