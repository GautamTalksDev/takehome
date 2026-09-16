/**
 * Cross-jurisdiction M-003 probes: exact half-cent T2/P where float≠decimal.
 * Records up vs down; float is not the general PDOC rule.
 */
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { chromium } from "playwright";
import { fillSalaryForm, type CaptureInput } from "./capture.ts";
import { prepareSession, USER_AGENT, findRepoRoot } from "./pdoc.ts";

type Probe = {
  province: string;
  pay_period: number;
  gross_pay: string;
  federal_claim_code: number;
  provincial_claim_code: number;
  t2: string;
  engine_provincial: string;
  float_pred: string;
  federal: string;
};

async function main() {
  const repoRoot = findRepoRoot();
  const probes: Probe[] = JSON.parse(
    await readFile(join(repoRoot, "data/grids/halfcent-probes.json"), "utf8"),
  );
  const { limiter } = await prepareSession({ probe: true });
  const browser = await chromium.launch({ headless: true });
  const results: Array<Record<string, unknown>> = [];

  try {
    for (const p of probes) {
      await limiter.waitTurn();
      const context = await browser.newContext({ userAgent: USER_AGENT });
      const page = await context.newPage();
      try {
        const input: CaptureInput = {
          province: p.province,
          payPeriod: p.pay_period,
          grossPay: p.gross_pay,
          datePaid: "2026-07-01",
          federalClaimCode: p.federal_claim_code,
          provincialClaimCode: p.provincial_claim_code,
        };
        const out = await fillSalaryForm(page, input);
        const matchesFloat = out.provincial_tax === p.float_pred;
        const matchesEngine = out.provincial_tax === p.engine_provincial;
        const row = {
          ...p,
          pdoc_provincial: out.provincial_tax,
          pdoc_federal: out.federal_tax,
          matches_float_pred: matchesFloat,
          matches_engine: matchesEngine,
        };
        results.push(row);
        console.log(
          `${p.province} P=${p.pay_period} ${p.gross_pay}: pdoc=${out.provincial_tax} eng=${p.engine_provincial} float=${p.float_pred} → ${matchesFloat ? "FLOAT" : matchesEngine ? "ENGINE" : "OTHER"}`,
        );
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
  }

  const floatN = results.filter((r) => r.matches_float_pred).length;
  const engN = results.filter((r) => r.matches_engine).length;
  const other = results.length - floatN - engN;
  const unique = new Set(
    results.map(
      (r) =>
        `${r.province}|${r.pay_period}|${r.gross_pay}|${r.federal_claim_code}|${r.provincial_claim_code}`,
    ),
  );
  const summary = {
    n: results.length,
    unique_forms: unique.size,
    match_float_pred: floatN,
    match_engine: engN,
    other,
    nl_float: results.filter((r) => r.province === "NL" && r.matches_float_pred).length,
    nl_engine: results.filter((r) => r.province === "NL" && r.matches_engine).length,
    nl_n: results.filter((r) => r.province === "NL").length,
    by_province: Object.fromEntries(
      [...new Set(results.map((r) => String(r.province)))].map((prov) => [
        prov,
        {
          float: results.filter((r) => r.province === prov && r.matches_float_pred).length,
          engine: results.filter((r) => r.province === prov && r.matches_engine).length,
          n: results.filter((r) => r.province === prov).length,
        },
      ]),
    ),
    results,
  };
  const outPath = join(repoRoot, "data/grids/halfcent-probe-results.json");
  await writeFile(outPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`\nSummary: ${floatN}/${results.length} match float_pred, ${engN} match engine, ${other} other; unique=${unique.size}`);
  console.log(`wrote ${outPath}`);
  // Float-as-general-PDOC-rule is falsified. Do not fail the run on a low
  // float hit rate — the artifact is the per-province split (especially NL).
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
