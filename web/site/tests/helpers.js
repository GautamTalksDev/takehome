import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO = path.resolve(HERE, '../../..');

export const M1_ON_WEEKLY_1000 = {
  as_of: '2026-01-15',
  province: 'ON',
  pay_period: 52,
  gross_pay: '1000.00',
  federal_claim_code: 1,
  provincial_claim_code: 1,
  cpp_months: 12,
};

export function nativeCli() {
  const candidates = [
    process.env.TAKEHOME_CLI,
    path.join(REPO, 'target/release/takehome'),
    path.join(REPO, 'target/debug/takehome'),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error('native takehome CLI not built; cargo build -p takehome-cli --release');
}

export function engineCalculate(request) {
  const result = spawnSync(nativeCli(), ['calculate'], {
    input: JSON.stringify(request),
    encoding: 'utf8',
    maxBuffer: 8 * 1024 * 1024,
  });
  if (result.status !== 0) {
    throw new Error(`native calculate failed: ${result.stderr || result.error}`);
  }
  return JSON.parse(result.stdout);
}

export async function fillKnownOntarioWeekly(page, gross = '1000.00') {
  await page.locator('[data-testid="as-of"]').fill(M1_ON_WEEKLY_1000.as_of);
  await page.locator('[data-testid="pay-period"]').selectOption('52');
  await page.locator('[data-testid="federal-claim"]').selectOption('1');
  await page.locator('[data-testid="provincial-claim"]').selectOption('1');
  await page.locator('[data-testid="gross-pay"]').fill(gross);
}

export async function waitForEngine(page) {
  await page.locator('#calculator[data-ready="true"]').waitFor({ timeout: 30_000 });
}
