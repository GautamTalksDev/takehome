/**
 * Playwright form driver for CRA PDOC salary calculations (Appendix P).
 * Never writes engine output into expected.
 *
 * Native radio/checkbox inputs are often aria-hidden; prefer getByText /
 * getByLabel and force where Angular hides the control.
 */

import type { Page } from "playwright";
import {
  APPENDIX_P_CAPTURED_AT,
  claimCodeValue,
  LIVE_RULE_SET_VERSION,
  PAY_PERIOD_VALUE,
  PROVINCE_VALUE,
  RESULTS,
  STEP3,
} from "./appendix-p.ts";
import { PolicyError } from "./pdoc.ts";

export type CaptureInput = {
  province: string;
  payPeriod: number;
  grossPay: string;
  datePaid: string;
  federalClaimCode: number | "E";
  provincialClaimCode: number | "E";
  cppMonths?: number;
  bonus?: string | null;
  ytdCpp?: string | null;
  ytdCpp2?: string | null;
  ytdPensionableEarnings?: string | null;
  ytdEi?: string | null;
  ytdInsurableEarnings?: string | null;
  additionalTax?: string | null;
};

export type CaptureOutput = {
  federal_tax: string;
  provincial_tax: string;
  cpp: string;
  cpp2: string;
  ei: string;
  total_deductions: string;
  net_pay: string;
};

export function mapPayPeriod(payPeriod: number): string {
  const v = PAY_PERIOD_VALUE[payPeriod];
  if (!v) {
    throw new PolicyError(
      `Pay period P=${payPeriod} has no PDOC salary option (Appendix P). Uncapturable on salary form.`,
    );
  }
  return v;
}

export function mapProvince(province: string): string {
  const v = PROVINCE_VALUE[province];
  if (!v) {
    throw new PolicyError(`Unknown province code for PDOC: ${province}`);
  }
  return v;
}

function parseDatePaid(iso: string): { year: string; month: string; day: string } {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!m) throw new PolicyError(`Bad datePaid ${iso}`);
  return { year: m[1]!, month: m[2]!, day: m[3]! };
}

function moneyFromText(raw: string): string {
  const cleaned = raw.replace(/[^0-9.\-]/g, "");
  if (!cleaned) throw new PolicyError(`Could not parse money from ${JSON.stringify(raw)}`);
  const n = Number(cleaned);
  if (!Number.isFinite(n)) {
    throw new PolicyError(`Could not parse money from ${JSON.stringify(raw)}`);
  }
  return n.toFixed(2);
}

async function clickNamed(page: Page, name: string): Promise<void> {
  const btn = page.getByRole("button", { name, exact: true });
  if ((await btn.count()) > 0) {
    await btn.first().click({ force: true, timeout: 30_000 });
  } else {
    await page.getByText(name, { exact: true }).first().click({ force: true, timeout: 30_000 });
  }
  await page.waitForLoadState("domcontentloaded");
}

async function waitStep(page: Page, step: RegExp, ready?: RegExp): Promise<void> {
  await page.waitForURL(step, { timeout: 60_000 });
  if (ready) {
    await page.getByText(ready).first().waitFor({ state: "visible", timeout: 30_000 });
  }
}

async function selectLabeled(page: Page, label: RegExp, value: string): Promise<void> {
  await page.getByLabel(label).selectOption(value);
}

async function fillLabeled(page: Page, label: RegExp, value: string): Promise<void> {
  const box = page.getByLabel(label).first();
  await box.click({ force: true });
  await box.fill("");
  await box.pressSequentially(value, { delay: 15 });
  await box.blur();
}

export async function fillSalaryForm(
  page: Page,
  input: CaptureInput,
): Promise<CaptureOutput> {
  const province = mapProvince(input.province);
  const payPeriod = mapPayPeriod(input.payPeriod);
  const date = parseDatePaid(input.datePaid);

  await page.goto("https://apps.cra-arc.gc.ca/ebci/rhpd/beta/entry", {
    waitUntil: "domcontentloaded",
  });
  await page.waitForFunction(() => document.querySelector('input[name="calculationType"]'));
  await clickNamed(page, "Salary");
  await clickNamed(page, "Next");
  await waitStep(page, /\/step1/, /Province or territory of employment/i);

  await page.getByLabel(/Province or territory of employment/i).waitFor({ state: "visible" });
  await selectLabeled(page, /Province or territory of employment/i, province);
  await selectLabeled(page, /Pay period frequency/i, payPeriod);
  await page.locator("#datePaidYear").selectOption(date.year);
  // Month select has January…December; day select has 01…31. Do not key off
  // numeric option values alone — day "12" collides with month value "12".
  const selects = page.locator("select");
  const count = await selects.count();
  for (let i = 0; i < count; i++) {
    const sel = selects.nth(i);
    const id = await sel.getAttribute("id");
    if (id === "datePaidYear") continue;
    const labels = await sel.locator("option").evaluateAll((opts) =>
      opts.map((o) => (o.textContent || "").trim()),
    );
    if (labels.includes("January") && labels.includes("December")) {
      await sel.selectOption(date.month);
    } else if (
      labels.includes("Day") ||
      (labels.includes("01") && labels.includes("28") && !labels.includes("January"))
    ) {
      await sel.selectOption(date.day);
    }
  }
  await clickNamed(page, "Next");
  await waitStep(page, /\/step2/, /Salary or wages income per pay period/i);

  await fillLabeled(page, /Salary or wages income per pay period/i, input.grossPay);
  if (input.bonus) {
    await clickNamed(page, "A bonus payment");
  } else {
    await clickNamed(page, "No bonus or retroactive payment");
  }
  await clickNamed(page, "Next");
  await waitStep(page, /\/step3/, /Claim codes|TD1 form/i);

  // §11.1 — claim codes, not fixed TD1 dollars.
  await page
    .locator(`input[name="td1ClaimCodeType"][value="${STEP3.td1ClaimCodes.value}"]`)
    .check({ force: true });
  await selectLabeled(
    page,
    /federal Form TD1 using claim codes/i,
    claimCodeValue(input.federalClaimCode),
  );
  // Outside Canada has no provincial/territorial TD1 claim-code select.
  if (input.province !== "OutsideCanada") {
    await selectLabeled(
      page,
      /provincial or territorial Form TD1 using claim codes/i,
      claimCodeValue(input.provincialClaimCode),
    );
  }
  if (input.additionalTax) {
    await fillLabeled(
      page,
      /Requested additional tax deductions from Form TD1/i,
      input.additionalTax,
    );
  }

  await page
    .locator(`input[name="cppQppType"][value="${STEP3.cppYtd.value}"]`)
    .check({ force: true });
  if (input.cppMonths != null) {
    await fillLabeled(page, /Number of pensionable months/i, String(input.cppMonths));
  }
  if (input.ytdPensionableEarnings) {
    await fillLabeled(
      page,
      /Pensionable earnings year-to-date/i,
      input.ytdPensionableEarnings,
    );
  }
  if (input.ytdCpp) {
    await fillLabeled(page, /CPP contributions deducted year-to-date/i, input.ytdCpp);
  }
  if (input.ytdCpp2) {
    await fillLabeled(
      page,
      /Second additional CPP contributions deducted year-to-date/i,
      input.ytdCpp2,
    );
  }

  await page
    .locator(`input[name="employmentInsuranceType"][value="${STEP3.eiYtd.value}"]`)
    .check({ force: true });
  if (input.ytdInsurableEarnings) {
    await fillLabeled(
      page,
      /Insurable earnings year-to-date/i,
      input.ytdInsurableEarnings,
    );
  }
  if (input.ytdEi) {
    await fillLabeled(page, /EI premiums deducted year-to-date/i, input.ytdEi);
  }

  // Prefer the calculate API payload — the results DOM sometimes crashes
  // mid-render (Angular stack overflow) even when the API succeeded.
  const calcResponse = page.waitForResponse(
    (res) =>
      res.url().includes("/SALARY/calculate") &&
      !res.url().includes("Claim") &&
      res.status() === 200,
    { timeout: 60_000 },
  );
  await clickNamed(page, "Calculate");
  const apiRes = await calcResponse;
  const raw = await apiRes.text();
  const json = JSON.parse(raw.replace(/^\)\]\}',?\s*/, ""));
  await page.waitForURL(/\/results/, { timeout: 60_000 }).catch(() => undefined);
  return outputFromCalculateApi(json);
}

function moneyField(v: unknown): string {
  if (typeof v === "number") return v.toFixed(2);
  if (typeof v === "string") return moneyFromText(v);
  throw new PolicyError(`Expected money field, got ${typeof v}`);
}

export function outputFromCalculateApi(json: Record<string, unknown>): CaptureOutput {
  const federal = moneyField(json.federalTaxDeduction);
  const additional = moneyField(json.requestedAdditionalTaxDeductions ?? 0);
  // Match M1 mapping: federal line includes additional tax L.
  const fedTotal = moneyFromText(
    (Number(federal) + Number(additional)).toFixed(2),
  );
  return {
    federal_tax: fedTotal,
    provincial_tax: moneyField(json.provincialTaxDeduction),
    cpp: moneyField(json.cppOrQppDeductions),
    cpp2: moneyField(json.secondCppOrQppDeductions ?? 0),
    ei: moneyField(json.employmentInsuranceDeductions),
    total_deductions: moneyField(json.totalDeductions),
    net_pay: moneyField(json.netAmount),
  };
}

async function lineAmount(page: Page, label: RegExp): Promise<string | null> {
  const text = await page.locator("body").innerText();
  const lines = text
    .split(/\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  for (let i = 0; i < lines.length; i++) {
    if (label.test(lines[i]!)) {
      const same = lines[i]!.replace(label, "").trim();
      if (/[\d]/.test(same)) return moneyFromText(same);
      if (i + 1 < lines.length && /[\d]/.test(lines[i + 1]!)) {
        return moneyFromText(lines[i + 1]!);
      }
    }
  }
  return null;
}

export async function readResults(page: Page): Promise<CaptureOutput> {
  const federal = await lineAmount(page, RESULTS.federalTax);
  const additional = (await lineAmount(page, RESULTS.additionalTax)) ?? "0.00";
  const provincial = await lineAmount(page, RESULTS.provincialTax);
  const cpp = await lineAmount(page, RESULTS.cpp);
  const cpp2 = (await lineAmount(page, RESULTS.cpp2)) ?? "0.00";
  const ei = await lineAmount(page, RESULTS.ei);
  const total = await lineAmount(page, RESULTS.totalDeductions);
  const net = await lineAmount(page, RESULTS.netAmount);

  const missing = [
    ["federal_tax", federal],
    ["provincial_tax", provincial],
    ["cpp", cpp],
    ["ei", ei],
    ["total_deductions", total],
    ["net_pay", net],
  ].filter(([, v]) => v == null);
  if (missing.length) {
    throw new PolicyError(
      `Results page missing lines: ${missing.map(([k]) => k).join(", ")}. Appendix P results locators may need refresh (captured ${APPENDIX_P_CAPTURED_AT}). Body sample: ${(await page.locator("body").innerText()).slice(0, 800)}`,
    );
  }

  const fed = moneyFromText((Number(federal) + Number(additional)).toFixed(2));

  return {
    federal_tax: fed,
    provincial_tax: provincial!,
    cpp: cpp!,
    cpp2,
    ei: ei!,
    total_deductions: total!,
    net_pay: net!,
  };
}

export { LIVE_RULE_SET_VERSION };
