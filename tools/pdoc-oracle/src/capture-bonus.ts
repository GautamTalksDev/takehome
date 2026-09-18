/**
 * Capture twenty bonus vectors from live PDOC into
 * crates/takehome-core/tests/vectors/pdoc_bonus_2026.json.
 * Never writes engine output into expected.
 */

import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { chromium } from "playwright";
import { LIVE_RULE_SET_VERSION } from "./appendix-p.ts";
import { fillSalaryForm, type CaptureInput } from "./capture.ts";
import {
  findRepoRoot,
  PolicyError,
  prepareSession,
  USER_AGENT,
} from "./pdoc.ts";

type VectorFile = {
  corpus: string;
  notes: string;
  vectors: Array<{
    id: string;
    description: string;
    request: Record<string, unknown>;
    expected: Record<string, string>;
    oracle: Record<string, string>;
  }>;
};

function claim(v: unknown): number | "E" {
  if (v === "E" || v === "e") return "E";
  return Number(v ?? 1);
}

export async function captureBonus(): Promise<{
  captured: number;
  pending: number;
  failed: string[];
}> {
  const repoRoot = findRepoRoot();
  const path = join(
    repoRoot,
    "crates/takehome-core/tests/vectors/pdoc_bonus_2026.json",
  );
  const file = JSON.parse(await readFile(path, "utf8")) as VectorFile;
  if (file.vectors.length !== 20) {
    throw new PolicyError(
      `bonus corpus must be 20 vectors, got ${file.vectors.length}`,
    );
  }

  const session = await prepareSession({ probe: true });
  const browser = await chromium.launch({ headless: true });
  const failed: string[] = [];
  let captured = 0;

  try {
    for (const v of file.vectors) {
      if (
        v.expected.federal_tax !== "PENDING_PDOC" &&
        v.oracle.retrieved_at !== "PENDING"
      ) {
        captured += 1;
        continue;
      }
      await session.limiter.waitTurn();
      const context = await browser.newContext({ userAgent: USER_AGENT });
      const page = await context.newPage();
      try {
        const input: CaptureInput = {
          province: String(v.request.province),
          payPeriod: Number(v.request.pay_period),
          grossPay: String(v.request.gross_pay),
          datePaid: String(v.request.as_of),
          federalClaimCode: claim(v.request.federal_claim_code),
          provincialClaimCode: claim(v.request.provincial_claim_code),
          cppMonths: 12,
          bonus: String(v.request.bonus),
        };
        const got = await fillSalaryForm(page, input);
        v.expected = {
          federal_tax: got.federal_tax,
          provincial_tax: got.provincial_tax,
          cpp: got.cpp,
          cpp2: got.cpp2,
          ei: got.ei,
          total_deductions: got.total_deductions,
          net_pay: got.net_pay,
        };
        v.oracle = {
          ...v.oracle,
          retrieved_at: new Date().toISOString(),
          pdoc_version_string: session.probe?.pdocIdentity ?? "",
          observed_edition: LIVE_RULE_SET_VERSION,
          browser: `chromium ${browser.version()}`,
          operator: "pdoc-oracle-harness",
        };
        captured += 1;
        console.error(`captured ${v.id}`);
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        failed.push(`${v.id}: ${msg}`);
        console.error(`failed ${v.id}: ${msg}`);
      } finally {
        await context.close();
      }
    }
  } finally {
    await browser.close();
  }

  await writeFile(path, `${JSON.stringify(file, null, 2)}\n`);
  return {
    captured,
    pending: file.vectors.filter((v) => v.expected.federal_tax === "PENDING_PDOC")
      .length,
    failed,
  };
}

const isDirect = process.argv[1]?.includes("capture-bonus");
if (isDirect) {
  captureBonus()
    .then((r) => {
      console.log(JSON.stringify(r, null, 2));
      if (r.failed.length) process.exit(1);
    })
    .catch((err: unknown) => {
      console.error(err instanceof Error ? err.stack ?? err.message : err);
      process.exit(1);
    });
}
