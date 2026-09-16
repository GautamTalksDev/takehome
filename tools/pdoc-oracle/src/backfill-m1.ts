/**
 * Import the twenty M1 Ontario vectors into data/pdoc-cache/records with
 * observedEdition = 2026-07-01 (live PDOC was serving the 123rd edition when
 * they were captured in September 2026). ruleSetVersion stays 2026-01-01.
 */

import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { LIVE_RULE_SET_VERSION } from "./appendix-p.ts";
import { assertRecordSatisfiesCase } from "./edition.ts";
import {
  cacheKey,
  canonicalJson,
  findRepoRoot,
  writeRecord,
  type CacheRecord,
} from "./pdoc.ts";

type VectorFile = {
  vectors: Array<{
    id: string;
    request: Record<string, unknown>;
    expected: Record<string, string>;
    oracle: {
      retrieved_at: string;
      pdoc_version_string?: string;
      browser: string;
      operator: string;
      screenshot_sha256?: string;
      observed_edition?: string;
    };
  }>;
};

export async function backfillM1(): Promise<{
  written: number;
  keys: string[];
}> {
  const repoRoot = findRepoRoot();
  const path = join(
    repoRoot,
    "crates/netpay-core/tests/vectors/pdoc_ontario_2026_01.json",
  );
  const file = JSON.parse(await readFile(path, "utf8")) as VectorFile;
  const keys: string[] = [];

  for (const v of file.vectors) {
    const ruleSetVersion = "2026-01-01";
    const observedEdition = v.oracle.observed_edition ?? LIVE_RULE_SET_VERSION;
    assertRecordSatisfiesCase({
      repoRoot,
      observedEdition,
      caseRuleSetVersion: ruleSetVersion,
      province: String(v.request.province ?? "ON"),
    });

    const key = cacheKey(v.request, ruleSetVersion);
    const record: CacheRecord = {
      key,
      ruleSetVersion,
      observedEdition,
      input: v.request,
      output: v.expected,
      retrievedAt: v.oracle.retrieved_at,
      pdocIdentity: v.oracle.pdoc_version_string ?? "unknown",
      robotsSha256: {},
      screenshotSha256: v.oracle.screenshot_sha256 ?? null,
      browser: v.oracle.browser,
      operator: v.oracle.operator === "manual" ? "manual" : "pdoc-oracle-harness",
    };
    await writeRecord(repoRoot, record);
    keys.push(key);
  }

  return { written: keys.length, keys };
}

const isDirect = process.argv[1]?.endsWith("backfill-m1.ts");
if (isDirect) {
  backfillM1()
    .then((r) => {
      console.log(canonicalJson(r));
    })
    .catch((err: unknown) => {
      console.error(err instanceof Error ? err.stack ?? err.message : err);
      process.exit(1);
    });
}
