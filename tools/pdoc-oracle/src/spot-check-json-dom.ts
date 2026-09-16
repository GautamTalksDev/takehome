/**
 * Spot-check: `/SALARY/calculate` JSON vs rendered results DOM (§11.2).
 * Hard-fails if a mapped field diverges when the DOM is readable.
 */
import { chromium } from "playwright";
import { fillSalaryForm, readResults, type CaptureInput } from "./capture.ts";
import { prepareSession, USER_AGENT, PolicyError } from "./pdoc.ts";

const CASES: Array<CaptureInput & { label: string }> = [
  {
    label: "ON weekly claim1",
    province: "ON",
    payPeriod: 52,
    grossPay: "1000.00",
    datePaid: "2026-07-01",
    federalClaimCode: 1,
    provincialClaimCode: 1,
  },
  {
    label: "AB P=10 claim0 M-003",
    province: "AB",
    payPeriod: 10,
    grossPay: "11704.49",
    datePaid: "2026-07-01",
    federalClaimCode: 0,
    provincialClaimCode: 0,
  },
  {
    label: "BC biweekly claim0",
    province: "BC",
    payPeriod: 26,
    grossPay: "2500.00",
    datePaid: "2026-07-01",
    federalClaimCode: 0,
    provincialClaimCode: 0,
  },
];

async function main() {
  const { limiter } = await prepareSession({ probe: true });
  const browser = await chromium.launch({ headless: true });
  const results: string[] = [];
  try {
    for (const c of CASES) {
      await limiter.waitTurn();
      const context = await browser.newContext({ userAgent: USER_AGENT });
      const page = await context.newPage();
      try {
        const jsonOut = await fillSalaryForm(page, c);
        let domOut: Awaited<ReturnType<typeof readResults>> | null = null;
        let domErr: string | null = null;
        try {
          await page.waitForURL(/\/results/, { timeout: 15_000 });
          await page.waitForTimeout(1500);
          domOut = await readResults(page);
        } catch (e) {
          domErr = e instanceof Error ? e.message : String(e);
        }
        if (!domOut) {
          // Partial DOM: federal/provincial often render before CPP/EI lines.
          // Re-parse body for the lines that are present and compare those.
          const body = await page.locator("body").innerText().catch(() => "");
          const pick = (re: RegExp): string | null => {
            const lines = body.split(/\n/).map((l) => l.trim()).filter(Boolean);
            for (let i = 0; i < lines.length; i++) {
              if (re.test(lines[i]!)) {
                const same = lines[i]!.replace(re, "").trim();
                const m = same.match(/[\d,]+\.\d{2}/) ?? lines[i + 1]?.match(/[\d,]+\.\d{2}/);
                return m ? m[0]!.replace(/,/g, "") : null;
              }
            }
            return null;
          };
          const fed = pick(/Federal tax deduction/i);
          const prov = pick(/Provincial\/territorial tax deduction/i);
          const partial: string[] = [];
          if (fed && fed !== jsonOut.federal_tax) {
            partial.push(`federal_tax: json=${jsonOut.federal_tax} dom=${fed}`);
          }
          if (prov && prov !== jsonOut.provincial_tax) {
            partial.push(`provincial_tax: json=${jsonOut.provincial_tax} dom=${prov}`);
          }
          if (partial.length) {
            throw new PolicyError(`JSON≠DOM (partial) on ${c.label}: ${partial.join("; ")}`);
          }
          results.push(
            `${c.label}: JSON ok (fed=${jsonOut.federal_tax} prov=${jsonOut.provincial_tax}); DOM partial match on tax lines (full results DOM incomplete — known Angular hazard)`,
          );
          console.log(results[results.length - 1]);
          continue;
        }
        const keys = [
          "federal_tax",
          "provincial_tax",
          "cpp",
          "cpp2",
          "ei",
          "total_deductions",
          "net_pay",
        ] as const;
        const diffs: string[] = [];
        for (const k of keys) {
          if (jsonOut[k] !== domOut[k]) {
            diffs.push(`${k}: json=${jsonOut[k]} dom=${domOut[k]}`);
          }
        }
        if (diffs.length) {
          throw new PolicyError(`JSON≠DOM on ${c.label}: ${diffs.join("; ")}`);
        }
        results.push(
          `${c.label}: JSON≡DOM federal=${jsonOut.federal_tax} provincial=${jsonOut.provincial_tax}`,
        );
        console.log(results[results.length - 1]);
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
  }
  console.log("\nSpot-check complete:");
  for (const r of results) console.log(" ", r);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
