import { expect, test } from '@playwright/test';
import { fillKnownOntarioWeekly, waitForEngine } from './helpers.js';

test('no network during a calculation after load (spec §5.5 / test 9)', async ({
  page,
}) => {
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page, '1000.00');
  await waitForEngine(page);

  const before = await page.getByTestId('net-pay').innerText();
  const afterReady = [];
  page.on('request', (req) => {
    afterReady.push(req.url());
  });

  await page.getByTestId('gross-pay').fill('2500.00');
  await expect(page.getByTestId('net-pay')).not.toHaveText(before);

  expect(
    afterReady,
    `calculation issued network requests: ${afterReady.join(', ')}`,
  ).toEqual([]);
});
