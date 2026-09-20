import { expect, test } from '@playwright/test';
import {
  fillKnownOntarioWeekly,
  waitForEngine,
} from './helpers.js';

test('Sources is a citation list, not serialized factors.json', async ({
  page,
}) => {
  await page.goto('/calculators/on/');
  const sources = page.locator('section', { has: page.getByRole('heading', { name: 'Sources' }) });
  await expect(sources).toBeVisible();
  const before = await sources.innerText();
  expect(before).not.toContain('{');
  expect(before).not.toContain('"what":');
  expect(before).not.toMatch(/\\?"table\\?":/);

  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);

  const after = await sources.innerText();
  expect(after).not.toContain('{');
  expect(after).not.toContain('"what":');
  expect(after).not.toMatch(/\\?"table\\?":/);

  const links = sources.getByTestId('citations').locator('a[href^="http"]');
  await expect(links).not.toHaveCount(0);
  const hrefs = await links.evaluateAll((nodes) =>
    nodes.map((node) => node.getAttribute('href')),
  );
  expect(hrefs.every((href) => href && !href.endsWith('#'))).toBe(true);

  const r = links.filter({ hasText: 'R —' });
  await expect(r).toContainText('federal tax rate');
  await expect(r).toContainText('Table 8.1');
  await expect(r).toHaveAttribute('href', /#toc\d+/);
});
