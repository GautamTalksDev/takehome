import { expect, test } from '@playwright/test';
import { fillKnownOntarioWeekly, waitForEngine } from './helpers.js';

test('home page is a working calculator with no account wall', async ({
  page,
}) => {
  await page.goto('/');
  await fillKnownOntarioWeekly(page);
  await waitForEngine(page);
  await expect(page.getByTestId('net-pay')).not.toHaveText('');
  await expect(page.getByText(/sign up/i)).toHaveCount(0);
  await expect(page.getByText(/cookie banner/i)).toHaveCount(0);
  await expect(page.locator('[role="dialog"]')).toHaveCount(0);
});
