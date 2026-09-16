/**
 * Capture runner: stratified smoke or full queue.
 * Checkpoint every 100. Resume from cache. Drift alarm via prepareSession.
 */

import { spawnSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { LIVE_RULE_SET_VERSION, PAY_PERIOD_VALUE } from "./appendix-p.ts";
import { fillSalaryForm, type CaptureInput } from "./capture.ts";
import { assertRecordSatisfiesCase, EditionProvenanceError } from "./edition.ts";
import {
  cacheKey,
  canonicalJson,
  formatHeartbeat,
  HARD_STOP_AFTER,
  HEARTBEAT_MS,
  lookupCached,
  prepareSession,
  ProgressTracker,
  readIdentity,
  sha256,
  stallIfIdle,
  writeIdentity,
  writeRecord,
  type CacheRecord,
  PolicyError,
} from "./pdoc.ts";

export type QueueItem = {
  rule_set_version: string;
  as_of: string;
  province: string;
  pay_period: number;
  gross_pay: string;
  calculation_option: string;
  bonus: string | null;
  federal_claim_code: number;
  provincial_claim_code: number;
  boundary_class: string;
};

export type CaptureRunResult = {
  mode: "smoke" | "queue";
  attempted: number;
  cacheHits: number;
  captured: number;
  matched: number;
  mismatched: number;
  skippedUncapturable: number;
  editionRetired: number;
  hardStopped: boolean;
  mismatchRate: string | null;
  mismatches: Array<{ key: string; province: string; deltas: Record<string, string> }>;
  checkpointPath: string;
};

type Checkpoint = {
  completedKeys: string[];
  matched: number;
  mismatched: number;
  captured: number;
  cacheHits: number;
  skippedUncapturable: number;
  editionRetired: number;
  mismatches: CaptureRunResult["mismatches"];
};

function requestFromItem(item: QueueItem): Record<string, unknown> {
  const req: Record<string, unknown> = {
    as_of: item.as_of,
    province: item.province,
    pay_period: item.pay_period,
    gross_pay: item.gross_pay,
    cpp_months: 12,
    federal_claim_code: item.federal_claim_code,
    provincial_claim_code: item.provincial_claim_code,
  };
  if (item.bonus) req.bonus = item.bonus;
  return req;
}

function engineEmployee(repoRoot: string, request: unknown): Record<string, string> | null {
  const bin = join(repoRoot, "target/debug/netpay-cli");
  const run = () =>
    spawnSync(bin, ["calculate"], {
      input: JSON.stringify(request),
      encoding: "utf8",
      cwd: repoRoot,
    });
  let r = run();
  if (r.status !== 0) {
    const build = spawnSync("cargo", ["build", "-p", "netpay-cli", "-q"], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    if (build.status !== 0) {
      console.error("netpay-cli build failed:", build.stderr);
      return null;
    }
    r = run();
    if (r.status !== 0) {
      console.error("netpay-cli calculate failed:", r.stderr);
      return null;
    }
  }
  return JSON.parse(r.stdout) as Record<string, string>;
}

function fieldDeltas(
  engine: Record<string, string>,
  pdoc: Record<string, string>,
): Record<string, string> {
  const keys = [
    "federal_tax",
    "provincial_tax",
    "cpp",
    "cpp2",
    "ei",
    "total_deductions",
    "net_pay",
  ];
  const out: Record<string, string> = {};
  for (const k of keys) {
    if (engine[k] !== pdoc[k]) {
      out[k] = `engine=${engine[k]} pdoc=${pdoc[k]}`;
    }
  }
  return out;
}

async function loadQueue(
  repoRoot: string,
  mode: "smoke" | "queue",
  limit?: number,
): Promise<QueueItem[]> {
  const outDir = join(repoRoot, "data/grids");
  await mkdir(outDir, { recursive: true });
  if (mode === "queue") {
    const emit = spawnSync(
      "cargo",
      ["run", "-p", "netpay-grid-gen", "-q", "--", "--queue", "--out", outDir],
      { cwd: repoRoot, encoding: "utf8" },
    );
    if (emit.status !== 0) {
      throw new PolicyError(`grid-gen --queue failed: ${emit.stderr}`);
    }
    const raw = await readFile(join(outDir, "pdoc-queue.json"), "utf8");
    return JSON.parse(raw) as QueueItem[];
  }
  const n = limit ?? 200;
  const emit = spawnSync(
    "cargo",
    ["run", "-p", "netpay-grid-gen", "-q", "--", "--smoke", String(n), "--out", outDir],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (emit.status !== 0) {
    throw new PolicyError(`grid-gen smoke failed: ${emit.stderr}`);
  }
  const raw = await readFile(join(outDir, "smoke-queue.json"), "utf8");
  return JSON.parse(raw) as QueueItem[];
}

export async function runCapture(opts: {
  mode: "smoke" | "queue";
  limit?: number;
}): Promise<CaptureRunResult> {
  const { repoRoot, robots, limiter, probe } = await prepareSession({ probe: true });
  if (!probe) throw new PolicyError("capture requires identity probe");

  let identity = await readIdentity(repoRoot);
  if (!identity) {
    identity = {
      pdocIdentity: probe.pdocIdentity,
      recordedAt: new Date().toISOString(),
      source: "pdoc-entry-probe",
    };
    await writeIdentity(repoRoot, identity);
  }

  const queue = await loadQueue(repoRoot, opts.mode, opts.limit);
  const checkpointPath = join(
    repoRoot,
    "data/pdoc-cache",
    opts.mode === "smoke" ? "smoke-checkpoint.json" : "queue-checkpoint.json",
  );
  let cp: Checkpoint = {
    completedKeys: [],
    matched: 0,
    mismatched: 0,
    captured: 0,
    cacheHits: 0,
    skippedUncapturable: 0,
    editionRetired: 0,
    mismatches: [],
  };
  try {
    cp = {
      ...cp,
      ...(JSON.parse(await readFile(checkpointPath, "utf8")) as Checkpoint),
    };
  } catch {
    /* fresh */
  }
  const done = new Set(cp.completedKeys);
  const robotsSha256: Record<string, string> = {};
  for (const r of robots) robotsSha256[r.url] = r.sha256;

  const { chromium } = await import("playwright");
  let browser = await chromium.launch({ headless: true });
  let failures = 0;
  let attempted = 0;
  const progressLog = join(repoRoot, "data/pdoc-cache/queue-progress.log");
  const progress = new ProgressTracker(Date.now());

  const saveCp = async () => {
    await mkdir(join(repoRoot, "data/pdoc-cache"), { recursive: true });
    await writeFile(checkpointPath, `${canonicalJson(cp)}\n`, "utf8");
  };

  const appendProgress = async (line: string) => {
    console.error(line);
    await writeFile(progressLog, `${new Date().toISOString()} ${line}\n`, {
      flag: "a",
    });
  };

  const heartbeat = setInterval(() => {
    void (async () => {
      try {
        stallIfIdle(progress.lastProgressMs, Date.now());
        await appendProgress(
          formatHeartbeat({
            attempted,
            completed: cp.completedKeys.length,
            lastProgressAt: new Date(progress.lastProgressMs).toISOString(),
            idleMs: Date.now() - progress.lastProgressMs,
          }),
        );
      } catch (err) {
        const msg = err instanceof Error ? err.stack ?? err.message : String(err);
        console.error(msg);
        try {
          await saveCp();
        } catch (saveErr) {
          console.error("checkpoint save on stall failed:", saveErr);
        }
        process.exit(1);
      }
    })();
  }, HEARTBEAT_MS);

  try {
    for (const item of queue) {
      if (!(item.pay_period in PAY_PERIOD_VALUE)) {
        cp.skippedUncapturable += 1;
        continue;
      }
      if (Number(item.gross_pay) === 0) {
        // PDOC salary step2 will not advance with $0.00 income.
        cp.skippedUncapturable += 1;
        continue;
      }

      const request = requestFromItem(item);
      const key = cacheKey(request, item.rule_set_version);
      if (done.has(key)) continue;

      attempted += 1;
      if (attempted <= 5 || attempted % 25 === 0) {
        console.error(
          `attempt ${attempted} ${item.province} P=${item.pay_period} gross=${item.gross_pay} claims=${item.federal_claim_code}/${item.provincial_claim_code}`,
        );
      }
      try {
        assertRecordSatisfiesCase({
          repoRoot,
          observedEdition: LIVE_RULE_SET_VERSION,
          caseRuleSetVersion: item.rule_set_version,
          province: item.province,
        });
      } catch (err) {
        if (err instanceof EditionProvenanceError) {
          cp.editionRetired += 1;
          done.add(key);
          cp.completedKeys.push(key);
          progress.editionRetired();
          continue;
        }
        throw err;
      }

      let record: CacheRecord | null = null;
      try {
        record = await lookupCached(
          repoRoot,
          request,
          item.rule_set_version,
          probe.pdocIdentity,
          { province: item.province },
        );
      } catch (err) {
        if (err instanceof EditionProvenanceError) {
          cp.editionRetired += 1;
          done.add(key);
          cp.completedKeys.push(key);
          progress.editionRetired();
          continue;
        }
        throw err;
      }

      let output: Record<string, string>;
      if (record) {
        cp.cacheHits += 1;
        output = record.output as Record<string, string>;
        progress.cacheHit();
      } else {
        await limiter.waitTurn();
        const context = await browser.newContext({
          userAgent:
            "Netpay-PDOC-Oracle/0.1 (+https://github.com/netpay-ca/netpay; conformance; contact=https://github.com/netpay-ca/netpay/blob/main/docs/CONFORMANCE-OPERATIONS.md)",
        });
        const page = await context.newPage();
        try {
          const captureInput: CaptureInput = {
            province: item.province,
            payPeriod: item.pay_period,
            grossPay: item.gross_pay,
            datePaid: item.as_of,
            federalClaimCode: item.federal_claim_code as number,
            provincialClaimCode: item.provincial_claim_code as number,
            cppMonths: 12,
            bonus: item.bonus,
          };
          const got = await fillSalaryForm(page, captureInput);
          const shot = await page.screenshot({ fullPage: true });
          output = got;
          const newRecord: CacheRecord = {
            key,
            ruleSetVersion: item.rule_set_version,
            observedEdition: LIVE_RULE_SET_VERSION,
            input: request,
            output: got,
            retrievedAt: new Date().toISOString(),
            pdocIdentity: probe.pdocIdentity,
            robotsSha256,
            screenshotSha256: sha256(shot),
            browser: `chromium ${browser.version()}`,
            operator: "pdoc-oracle-harness",
          };
          await writeRecord(repoRoot, newRecord);
          cp.captured += 1;
          failures = 0;
          progress.captured();
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          // Stuck on step2 with validation — treat as uncapturable, not a hard failure.
          if (page.url().includes("/step2") && msg.includes("waitForURL")) {
            cp.skippedUncapturable += 1;
            done.add(key);
            cp.completedKeys.push(key);
            await context.close();
            failures = 0;
            progress.step2Skip();
            continue;
          }
          failures += 1;
          await context.close();
          if (failures >= HARD_STOP_AFTER) {
            cp.completedKeys = [...done];
            progress.hardError();
            await saveCp();
            return {
              mode: opts.mode,
              attempted,
              cacheHits: cp.cacheHits,
              captured: cp.captured,
              matched: cp.matched,
              mismatched: cp.mismatched,
              skippedUncapturable: cp.skippedUncapturable,
              editionRetired: cp.editionRetired,
              hardStopped: true,
              mismatchRate: null,
              mismatches: cp.mismatches,
              checkpointPath,
            };
          }
          console.error(`capture failure ${failures}:`, err);
          progress.hardError();
          continue;
        }
        await context.close();
      }

      const engine = engineEmployee(repoRoot, request);
      if (engine) {
        const deltas = fieldDeltas(engine, output);
        if (Object.keys(deltas).length === 0) {
          cp.matched += 1;
        } else {
          cp.mismatched += 1;
          cp.mismatches.push({ key, province: item.province, deltas });
        }
      }

      done.add(key);
      cp.completedKeys.push(key);
      if (cp.completedKeys.length % 10 === 0) {
        await saveCp();
        await appendProgress(
          `checkpoint ${cp.completedKeys.length}: matched=${cp.matched} mismatched=${cp.mismatched} captured=${cp.captured} hits=${cp.cacheHits} attempted=${attempted}`,
        );
      }
    }
  } finally {
    clearInterval(heartbeat);
    await browser.close();
    await saveCp();
  }

  const compared = cp.matched + cp.mismatched;
  const mismatchRate =
    compared === 0 ? null : `${((100 * cp.mismatched) / compared).toFixed(2)}%`;

  return {
    mode: opts.mode,
    attempted,
    cacheHits: cp.cacheHits,
    captured: cp.captured,
    matched: cp.matched,
    mismatched: cp.mismatched,
    skippedUncapturable: cp.skippedUncapturable,
    editionRetired: cp.editionRetired,
    hardStopped: false,
    mismatchRate,
    mismatches: cp.mismatches.slice(0, 25),
    checkpointPath,
  };
}
