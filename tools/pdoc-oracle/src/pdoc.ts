/**
 * CRA PDOC oracle harness (docs/CONFORMANCE-OPERATIONS.md).
 *
 * Fail closed if the operations doc is missing. Honour robots.txt. Cache
 * forever under sha256(input || rule_set_version). Alarm if PDOC's identity
 * drifts from the identity the cache was built under. Never write engine
 * output into expected.
 */

import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import {
  mkdir,
  readFile,
  writeFile,
} from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

export const USER_AGENT =
  "Takehome-PDOC-Oracle/0.1 (+https://github.com/takehome-ca/takehome; conformance; contact=https://github.com/takehome-ca/takehome/blob/main/docs/CONFORMANCE-OPERATIONS.md)";

export const PDOC_ORIGIN = "https://apps.cra-arc.gc.ca";
export const PDOC_ENTRY_PATH = "/ebci/rhpd/beta/entry";
export const PDOC_ENTRY_URL = `${PDOC_ORIGIN}${PDOC_ENTRY_PATH}`;
export const CANADA_CA_ROBOTS = "https://www.canada.ca/robots.txt";
export const PDOC_ROBOTS = `${PDOC_ORIGIN}/robots.txt`;

export const MIN_GAP_MS = 3_000;
export const HARD_STOP_AFTER = 3;
/** Progress log / stall check while a capture is in flight. */
export const HEARTBEAT_MS = 30_000;
/** No completed form for this long → non-zero exit. Hung browser is not a rate limit.
 * Must exceed max backoff (180s) plus one form attempt. */
export const STALL_AFTER_MS = 300_000;
const MAX_BACKOFF_MS = 180_000;

export const OPS_DOC_REL = "docs/CONFORMANCE-OPERATIONS.md";

export class PolicyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PolicyError";
  }
}

/** Queue worker made no progress. Distinct from a PDOC HTTP failure. */
export class StallError extends PolicyError {
  constructor(
    readonly idleMs: number,
    readonly lastProgressAt: string,
  ) {
    super(
      `no progress for ${idleMs}ms (limit ${STALL_AFTER_MS}ms). last progress at ${lastProgressAt}. Hung browser or dead worker — not a rate limit. Exiting non-zero.`,
    );
    this.name = "StallError";
  }
}

/** Throw if `nowMs - lastProgressAtMs` has reached the stall limit. */
export function stallIfIdle(
  lastProgressAtMs: number,
  nowMs: number,
  stallAfterMs = STALL_AFTER_MS,
): void {
  const idleMs = nowMs - lastProgressAtMs;
  if (idleMs >= stallAfterMs) {
    throw new StallError(idleMs, new Date(lastProgressAtMs).toISOString());
  }
}

export function formatHeartbeat(args: {
  attempted: number;
  completed: number;
  lastProgressAt: string;
  idleMs: number;
}): string {
  return `heartbeat attempted=${args.attempted} completed=${args.completed} last_progress=${args.lastProgressAt} idle_s=${Math.floor(args.idleMs / 1000)}`;
}

/** Every queue terminal that must tick the stall watchdog. */
export const PROGRESS_TERMINALS = [
  "captured",
  "cacheHit",
  "step2Skip",
  "editionRetired",
  "hardError",
] as const;

export type ProgressTerminal = (typeof PROGRESS_TERMINALS)[number];

/** Stall watchdog clock. Named methods so a forgotten path is a missing call, not a silent alias of `mark`. */
export class ProgressTracker {
  lastProgressMs: number;

  constructor(nowMs: number) {
    this.lastProgressMs = nowMs;
  }

  captured(nowMs = Date.now()): void {
    this.lastProgressMs = nowMs;
  }
  cacheHit(nowMs = Date.now()): void {
    this.lastProgressMs = nowMs;
  }
  step2Skip(nowMs = Date.now()): void {
    this.lastProgressMs = nowMs;
  }
  editionRetired(nowMs = Date.now()): void {
    this.lastProgressMs = nowMs;
  }
  hardError(nowMs = Date.now()): void {
    this.lastProgressMs = nowMs;
  }
}

export class RobotsDisallowError extends PolicyError {
  constructor(message: string) {
    super(message);
    this.name = "RobotsDisallowError";
  }
}

export class PdocIdentityDriftError extends PolicyError {
  constructor(
    readonly cachedIdentity: string,
    readonly observedIdentity: string,
  ) {
    super(
      `PDOC identity drifted. Cache was built under ${cachedIdentity}; this run observed ${observedIdentity}. The entire cache may need re-validation. Do not fetch the case queue. See docs/CONFORMANCE-OPERATIONS.md §10.2.`,
    );
    this.name = "PdocIdentityDriftError";
  }
}

export type RobotsRule = {
  type: "allow" | "disallow";
  pattern: string;
};

export type RobotsFetch = {
  url: string;
  finalUrl: string;
  status: number;
  body: string;
  sha256: string;
  fetchedAt: string;
  present: boolean;
};

export type RunConditions = {
  startedAt: string;
  userAgent: string;
  robots: Array<Omit<RobotsFetch, "body"> & { allowsPdoc: boolean }>;
  pdocIdentity: string | null;
};

export type CacheRecord = {
  key: string;
  ruleSetVersion: string;
  /** Calendar edition live PDOC was serving when this result was observed. */
  observedEdition: string;
  input: unknown;
  output: unknown;
  retrievedAt: string;
  pdocIdentity: string;
  robotsSha256: Record<string, string>;
  screenshotSha256: string | null;
  browser: string;
  operator: "pdoc-oracle-harness" | "manual";
};

export type PdocIdentityFile = {
  pdocIdentity: string;
  recordedAt: string;
  source: "pdoc-entry-probe";
};

export function sha256(data: string | Buffer): string {
  return createHash("sha256").update(data).digest("hex");
}

export function findRepoRoot(start = dirname(fileURLToPath(import.meta.url))): string {
  let dir = start;
  for (let i = 0; i < 12; i++) {
    if (existsSync(join(dir, OPS_DOC_REL)) && existsSync(join(dir, "crates/takehome-core"))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new PolicyError(
    `Fail closed: ${OPS_DOC_REL} not found walking up from ${start}. The harness will not run without the published operations policy.`,
  );
}

export function requireOpsDoc(repoRoot: string): string {
  const path = join(repoRoot, OPS_DOC_REL);
  if (!existsSync(path)) {
    throw new PolicyError(
      `Fail closed: missing ${path}. Publish the operations policy before contacting PDOC.`,
    );
  }
  return path;
}

export function canonicalJson(value: unknown): string {
  return JSON.stringify(sortValue(value));
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value && typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(obj).sort()) {
      out[key] = sortValue(obj[key]);
    }
    return out;
  }
  return value;
}

export function cacheKey(input: unknown, ruleSetVersion: string): string {
  return sha256(`${canonicalJson(input)}||${ruleSetVersion}`);
}

/** RFC 9309-ish parser: User-agent groups, Allow/Disallow, longest match wins. */
export function parseRobots(text: string): Map<string, RobotsRule[]> {
  const groups = new Map<string, RobotsRule[]>();
  let agents: string[] = [];
  let inAgents = true;

  const commitRule = (rule: RobotsRule) => {
    if (agents.length === 0) return;
    for (const agent of agents) {
      const list = groups.get(agent) ?? [];
      list.push(rule);
      groups.set(agent, list);
    }
  };

  for (const raw of text.split(/\r?\n/)) {
    const line = raw.replace(/#.*$/, "").trim();
    if (!line) continue;
    const colon = line.indexOf(":");
    if (colon < 0) continue;
    const field = line.slice(0, colon).trim().toLowerCase();
    const value = line.slice(colon + 1).trim();

    if (field === "user-agent") {
      if (!inAgents) {
        agents = [];
        inAgents = true;
      }
      agents.push(value.toLowerCase());
      if (!groups.has(value.toLowerCase())) groups.set(value.toLowerCase(), []);
      continue;
    }
    inAgents = false;
    if (field === "allow") commitRule({ type: "allow", pattern: value });
    if (field === "disallow") commitRule({ type: "disallow", pattern: value });
  }
  return groups;
}

export function rulesForAgent(
  groups: Map<string, RobotsRule[]>,
  userAgent: string,
): RobotsRule[] {
  const ua = userAgent.toLowerCase();
  let best: { agent: string; rules: RobotsRule[] } | null = null;
  for (const [agent, rules] of groups) {
    if (agent === "*") continue;
    if (ua.includes(agent)) {
      if (!best || agent.length > best.agent.length) best = { agent, rules };
    }
  }
  if (best) return best.rules;
  return groups.get("*") ?? [];
}

function globToRegExp(pattern: string): RegExp {
  let out = "^";
  for (let i = 0; i < pattern.length; i++) {
    const ch = pattern[i];
    if (ch === "*") out += ".*";
    else if (ch === "$" && i === pattern.length - 1) out += "$";
    else out += ch.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(out);
}

export function pathAllowed(rules: RobotsRule[], pathAndQuery: string): boolean {
  const path = pathAndQuery.split("#")[0] ?? pathAndQuery;
  let winner: { len: number; type: "allow" | "disallow" } | null = null;
  for (const rule of rules) {
    if (rule.pattern === "") {
      // Empty Disallow/Allow: no restriction from this rule.
      continue;
    }
    const re = globToRegExp(rule.pattern);
    if (!re.test(path)) continue;
    const len = rule.pattern.replace(/\$$/, "").length;
    if (!winner || len > winner.len) winner = { len, type: rule.type };
  }
  if (!winner) return true;
  return winner.type === "allow";
}

export function pdocPathsToCheck(): string[] {
  return [
    PDOC_ENTRY_PATH,
    "/ebci/rhpd/beta/",
    "/ebci/rhpd/beta/results",
    "/en/revenue-agency/services/e-services/e-services-businesses/payroll-deductions-online-calculator.html",
  ];
}

export function originAllowsPdoc(fetch: RobotsFetch, origin: string): boolean {
  if (!fetch.present) return true;
  const groups = parseRobots(fetch.body);
  const rules = rulesForAgent(groups, USER_AGENT);
  const originUrl = new URL(origin);
  for (const path of pdocPathsToCheck()) {
    // Only apply path checks that belong on this origin.
    if (originUrl.hostname === "apps.cra-arc.gc.ca" && path.startsWith("/en/")) {
      continue;
    }
    if (originUrl.hostname.endsWith("canada.ca") && path.startsWith("/ebci/")) {
      continue;
    }
    if (!pathAllowed(rules, path)) return false;
  }
  return true;
}

export async function fetchRobots(url: string): Promise<RobotsFetch> {
  const fetchedAt = new Date().toISOString();
  const res = await fetch(url, {
    headers: { "user-agent": USER_AGENT, accept: "text/plain,*/*" },
    redirect: "follow",
  });
  const body = await res.text();
  const contentType = res.headers.get("content-type") ?? "";
  const looksLikeHtml =
    contentType.includes("text/html") || /^\s*<(!doctype|html)/i.test(body);
  const present = res.status === 200 && !looksLikeHtml;
  return {
    url,
    finalUrl: res.url || url,
    status: res.status,
    body,
    sha256: sha256(body),
    fetchedAt,
    present,
  };
}

export async function checkRobotsOrThrow(): Promise<RobotsFetch[]> {
  const fetches = [];
  for (const url of [CANADA_CA_ROBOTS, PDOC_ROBOTS]) {
    const got = await fetchRobots(url);
    if (got.status >= 500 || got.status === 429) {
      throw new PolicyError(
        `Fail closed: ${url} returned ${got.status}. An outage is not permission.`,
      );
    }
    fetches.push(got);
  }
  const canada = fetches[0]!;
  const pdoc = fetches[1]!;
  if (!originAllowsPdoc(canada, "https://www.canada.ca")) {
    throw new RobotsDisallowError(
      `${CANADA_CA_ROBOTS} disallows a PDOC path (sha256 ${canada.sha256}). Stop. Do not crawl around it.`,
    );
  }
  if (!originAllowsPdoc(pdoc, PDOC_ORIGIN)) {
    throw new RobotsDisallowError(
      `${PDOC_ROBOTS} disallows PDOC (status ${pdoc.status}, sha256 ${pdoc.sha256}). Stop.`,
    );
  }
  return fetches;
}

export function backoffMs(failureNumber: number): number {
  const n = Math.max(1, failureNumber);
  return Math.min(MIN_GAP_MS * 2 ** (n - 1), MAX_BACKOFF_MS);
}

export class RateLimiter {
  private nextOk = 0;
  constructor(private readonly gapMs = MIN_GAP_MS) {}

  async waitTurn(): Promise<void> {
    const now = Date.now();
    const wait = Math.max(0, this.nextOk - now);
    if (wait > 0) await delay(wait);
    this.nextOk = Date.now() + this.gapMs;
  }
}

export function cacheDir(repoRoot: string): string {
  return join(repoRoot, "data/pdoc-cache");
}

export function identityPath(repoRoot: string): string {
  return join(cacheDir(repoRoot), "pdoc-identity.json");
}

export function recordPath(repoRoot: string, key: string): string {
  return join(cacheDir(repoRoot), "records", `${key}.json`);
}

export async function readIdentity(
  repoRoot: string,
): Promise<PdocIdentityFile | null> {
  const path = identityPath(repoRoot);
  if (!existsSync(path)) return null;
  return JSON.parse(await readFile(path, "utf8")) as PdocIdentityFile;
}

export async function writeIdentity(
  repoRoot: string,
  identity: PdocIdentityFile,
): Promise<void> {
  await mkdir(cacheDir(repoRoot), { recursive: true });
  await writeFile(identityPath(repoRoot), `${canonicalJson(identity)}\n`, "utf8");
}

export async function readRecord(
  repoRoot: string,
  key: string,
): Promise<CacheRecord | null> {
  const path = recordPath(repoRoot, key);
  if (!existsSync(path)) return null;
  return JSON.parse(await readFile(path, "utf8")) as CacheRecord;
}

export async function writeRecord(
  repoRoot: string,
  record: CacheRecord,
): Promise<void> {
  const path = recordPath(repoRoot, record.key);
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${canonicalJson(record)}\n`, "utf8");
}

export function assertIdentityMatchesCache(
  cached: PdocIdentityFile | null,
  observed: string,
): void {
  if (!cached) return;
  if (cached.pdocIdentity !== observed) {
    throw new PdocIdentityDriftError(cached.pdocIdentity, observed);
  }
}

export function fingerprintFormStructure(
  controls: Array<{ name?: string; type?: string; label?: string }>,
): string {
  const rows = controls
    .map((c) => `${c.name ?? ""}|${c.type ?? ""}|${c.label ?? ""}`)
    .sort();
  return sha256(rows.join("\n"));
}

export function identityFromPage(opts: {
  versionString: string | null;
  formFingerprint: string;
}): string {
  return opts.versionString?.trim() || `form:${opts.formFingerprint}`;
}

export async function writeRunConditions(
  repoRoot: string,
  conditions: RunConditions,
): Promise<string> {
  const dir = cacheDir(repoRoot);
  await mkdir(dir, { recursive: true });
  const path = join(dir, "last-run-conditions.json");
  await writeFile(path, `${canonicalJson(conditions)}\n`, "utf8");
  return path;
}

export type ProbeResult = {
  pdocIdentity: string;
  versionString: string | null;
  formFingerprint: string;
  browser: string;
};

/**
 * One PDOC navigation per session: read version / form fingerprint.
 * Counts as a PDOC request under the 3-second rule.
 */
export async function probePdocIdentity(
  limiter: RateLimiter,
): Promise<ProbeResult> {
  await limiter.waitTurn();
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({ headless: true });
  try {
    const context = await browser.newContext({
      userAgent: USER_AGENT,
      extraHTTPHeaders: { "user-agent": USER_AGENT },
    });
    const page = await context.newPage();
    const res = await page.goto(PDOC_ENTRY_URL, { waitUntil: "domcontentloaded" });
    if (!res || !res.ok()) {
      throw new PolicyError(
        `PDOC identity probe failed: ${res?.status() ?? "no response"} at ${PDOC_ENTRY_URL}`,
      );
    }
    const versionString = await page.evaluate(() => {
      const text = document.body?.innerText ?? "";
      const labeled = text.match(
        /(?:version|PDOC|last\s*updated|mise\s*à\s*jour)[^\d]{0,40}(\d{4}-\d{2}-\d{2})/i,
      );
      if (labeled?.[1]) return labeled[1];
      const dates = text.match(/\b20\d{2}-\d{2}-\d{2}\b/g);
      return dates?.[dates.length - 1] ?? null;
    });
    const controls = await page.evaluate(() => {
      const nodes = [
        ...document.querySelectorAll("input, select, textarea, button"),
      ];
      return nodes.map((el) => {
        const html = el as HTMLInputElement;
        const id = html.id;
        const label = id
          ? document.querySelector(`label[for="${CSS.escape(id)}"]`)?.textContent
          : null;
        return {
          name: html.name || html.id || html.getAttribute("aria-label") || "",
          type: html.type || el.tagName.toLowerCase(),
          label: (label || html.getAttribute("aria-label") || "").trim(),
        };
      });
    });
    const formFingerprint = fingerprintFormStructure(controls);
    const pdocIdentity = identityFromPage({ versionString, formFingerprint });
    const browserVersion = `chromium ${browser.version()}`;
    await context.close();
    return {
      pdocIdentity,
      versionString,
      formFingerprint,
      browser: browserVersion,
    };
  } finally {
    await browser.close();
  }
}

export async function prepareSession(opts?: {
  probe?: boolean;
}): Promise<{
  repoRoot: string;
  robots: RobotsFetch[];
  conditions: RunConditions;
  limiter: RateLimiter;
  probe: ProbeResult | null;
}> {
  const repoRoot = findRepoRoot();
  requireOpsDoc(repoRoot);
  const robots = await checkRobotsOrThrow();
  const limiter = new RateLimiter();
  let probe: ProbeResult | null = null;
  if (opts?.probe) {
    probe = await probePdocIdentity(limiter);
    const cached = await readIdentity(repoRoot);
    assertIdentityMatchesCache(cached, probe.pdocIdentity);
  }
  const conditions: RunConditions = {
    startedAt: new Date().toISOString(),
    userAgent: USER_AGENT,
    robots: robots.map((r) => ({
      url: r.url,
      finalUrl: r.finalUrl,
      status: r.status,
      sha256: r.sha256,
      fetchedAt: r.fetchedAt,
      present: r.present,
      allowsPdoc: originAllowsPdoc(
        r,
        r.url.includes("canada.ca") ? "https://www.canada.ca" : PDOC_ORIGIN,
      ),
    })),
    pdocIdentity: probe?.pdocIdentity ?? null,
  };
  await writeRunConditions(repoRoot, conditions);
  return { repoRoot, robots, conditions, limiter, probe };
}

export async function lookupCached(
  repoRoot: string,
  input: unknown,
  ruleSetVersion: string,
  observedIdentity: string | null,
  opts?: { province?: string },
): Promise<CacheRecord | null> {
  const key = cacheKey(input, ruleSetVersion);
  const record = await readRecord(repoRoot, key);
  if (!record) return null;
  if (observedIdentity && record.pdocIdentity !== observedIdentity) {
    throw new PdocIdentityDriftError(record.pdocIdentity, observedIdentity);
  }
  if (!record.observedEdition) {
    throw new PolicyError(
      `Cache record ${key} is missing observedEdition. Re-backfill or re-capture before use.`,
    );
  }
  const province =
    opts?.province ??
    (typeof input === "object" &&
    input &&
    "province" in (input as Record<string, unknown>)
      ? String((input as Record<string, unknown>).province)
      : "ON");
  const { assertRecordSatisfiesCase } = await import("./edition.ts");
  assertRecordSatisfiesCase({
    repoRoot,
    observedEdition: record.observedEdition,
    caseRuleSetVersion: ruleSetVersion,
    province,
  });
  return record;
}

async function main(argv: string[]): Promise<void> {
  const cmd = argv[0] ?? "help";
  if (cmd === "help" || cmd === "--help" || cmd === "-h") {
    console.log(`Usage:
  pdoc.ts check-robots     Fetch/parse robots.txt; exit 0 if PDOC is allowed
  pdoc.ts probe            robots.txt + one PDOC entry-page identity probe
  pdoc.ts backfill-m1      Import M1 twenty into cache with observedEdition
  pdoc.ts capture --smoke  Stratified 200-form smoke (Appendix P); report then stop
  pdoc.ts capture --queue  Capturable July PDOC queue (10 of 14 P; after smoke + republish)

Cache hits never contact PDOC. Identity probe is at most one PDOC request per run.
Never writes engine output into expected.`);
    return;
  }

  if (cmd === "check-robots") {
    const { conditions } = await prepareSession({ probe: false });
    console.log(canonicalJson(conditions));
    console.log("PDOC is allowed under current robots.txt.");
    return;
  }

  if (cmd === "probe") {
    const { conditions, probe } = await prepareSession({ probe: true });
    console.log(canonicalJson({ conditions, probe }));
    return;
  }

  if (cmd === "backfill-m1") {
    const { backfillM1 } = await import("./backfill-m1.ts");
    const result = await backfillM1();
    console.log(canonicalJson(result));
    return;
  }

  if (cmd === "capture") {
    const { runCapture } = await import("./run-capture.ts");
    const mode = argv.includes("--queue") ? "queue" : "smoke";
    const limitFlag = argv.findIndex((a) => a === "--limit");
    const limit =
      limitFlag >= 0 && argv[limitFlag + 1]
        ? Number(argv[limitFlag + 1])
        : mode === "smoke"
          ? 200
          : undefined;
    const result = await runCapture({ mode, limit });
    console.log(canonicalJson(result));
    return;
  }

  throw new PolicyError(`unknown command: ${cmd}`);
}

const isDirect =
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isDirect) {
  main(process.argv.slice(2)).catch((err: unknown) => {
    const message = err instanceof Error ? err.stack ?? err.message : String(err);
    console.error(message);
    process.exit(1);
  });
}
