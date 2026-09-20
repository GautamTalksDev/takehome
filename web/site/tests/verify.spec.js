import { expect, test } from '@playwright/test';

const CURL = `curl -sS -X POST \\
  https://takehome.gautamkhosla.com/v1/deductions \\
  -H 'content-type: application/json' \\
  -H 'authorization: Bearer np_test_0123456789abcdef0123456789abcdef' \\
  -d '{
  "as_of": "2026-01-15",
  "province": "ON",
  "pay_period": 52,
  "gross_pay": "1000.00",
  "federal_claim_code": 1,
  "provincial_claim_code": 1
}'`;

async function seedVerifiedUi(page) {
  await page.goto('/signup/verify/');
  await page.evaluate((curl) => {
    document.getElementById('verify-status').textContent =
      'Email verified. Use the test key while you integrate.';
    document.getElementById('verify-test-key').textContent =
      'np_test_0123456789abcdef0123456789abcdef';
    document.getElementById('verify-live-key').textContent =
      'np_live_0123456789abcdef0123456789abcdef';
    document.getElementById('verify-keys-panel').hidden = false;
    document.getElementById('verify-hint').hidden = false;
    document.getElementById('verify-curl').textContent = curl;
    document.getElementById('verify-curl-wrap').hidden = false;
  }, CURL);
}

test('verify curl fits without horizontal scroll at desktop and 380px', async ({
  page,
}) => {
  for (const width of [1280, 380]) {
    await page.setViewportSize({ width, height: 900 });
    await seedVerifiedUi(page);
    const metrics = await page.locator('#verify-curl').evaluate((el) => ({
      scrollWidth: el.scrollWidth,
      clientWidth: el.clientWidth,
    }));
    expect(
      metrics.scrollWidth,
      `width=${width} scrollWidth=${metrics.scrollWidth} clientWidth=${metrics.clientWidth}`,
    ).toBeLessThanOrEqual(metrics.clientWidth);
  }
});

test('verify page keeps a space before the net_pay figure', async ({ page }) => {
  await page.goto('/signup/verify/');
  const hint = page.locator('#verify-hint');
  await hint.evaluate((el) => {
    el.hidden = false;
  });
  await expect(hint).toContainText('must be 800.79');
  await expect(hint).not.toContainText('must be800.79');
});
