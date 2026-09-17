import { expect, test } from '@playwright/test';
import {
  engineCalculate,
  fillKnownOntarioWeekly,
  M1_ON_WEEKLY_1000,
  waitForEngine,
} from './helpers.js';

test('calculator works offline after first load (spec §5.5 / test 7)', async ({
  page,
  context,
}) => {
  await page.goto('/calculators/on/');
  await fillKnownOntarioWeekly(page, '1000.00');
  await waitForEngine(page);
  await page.evaluate(() => navigator.serviceWorker.ready);

  await context.setOffline(true);

  const next = { ...M1_ON_WEEKLY_1000, gross_pay: '2000.00' };
  const expected = engineCalculate(next);
  await page.getByTestId('gross-pay').fill('2000.00');
  await expect(page.getByTestId('net-pay')).toHaveText(expected.employee.net_pay);
  await expect(page.getByTestId('cpp')).toHaveText(expected.employee.cpp);
});
