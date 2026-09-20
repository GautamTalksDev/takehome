/**
 * Ruling (a): publish JSON PDOC records; keep PNG screenshots local;
 * bind screenshot SHA-256 in CONFORMANCE.md via a catalog digest.
 */
import {
  existsSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { canonicalJson, findRepoRoot, sha256 } from "./pdoc.ts";

export const SHA256_HEX = /^[0-9a-f]{64}$/;

export const CATALOG_REL = "data/pdoc-cache/screenshot-hashes.json";
export const RECORDS_REL = "data/pdoc-cache/records";
export const CORPUS_ARCHIVE_REL =
  "data/pdoc-cache/takehome-conformance-corpus-2026.1.tar.zst";
export const CORPUS_META_REL = "data/pdoc-cache/corpus-archive.json";
export const M1_VECTORS_REL =
  "crates/takehome-core/tests/vectors/pdoc_ontario_2026_01.json";

export type CorpusArchiveMeta = {
  filename: string;
  sha256: string;
  catalog_digest: string;
  download: string;
};

export type ScreenshotCatalog = {
  ruling: "records-only";
  source: string;
  hashes: Record<string, string>;
};

export type NamedHash = { id: string; sha256: string };

const IMAGE_MARKERS = [
  "screenshotBytes",
  "screenshot_png",
  "screenshotPng",
  "image/png",
  "iVBORw0KGgo",
];

/** SHA-256 of sorted `key<space>sha256\\n` lines. Not a pretty-print of the JSON. */
export function catalogDigest(hashes: Record<string, string>): string {
  const lines = Object.keys(hashes)
    .sort()
    .map((key) => `${key} ${hashes[key]}\n`)
    .join("");
  return sha256(lines);
}

export function hashesFromRecords(recordsDir: string): Record<string, string> {
  if (!existsSync(recordsDir)) {
    throw new Error(`records directory missing: ${recordsDir}`);
  }
  const hashes: Record<string, string> = {};
  for (const name of readdirSync(recordsDir)) {
    if (!name.endsWith(".json")) continue;
    const path = join(recordsDir, name);
    const raw = readFileSync(path, "utf8");
    const record = JSON.parse(raw) as {
      key?: string;
      screenshotSha256?: unknown;
    };
    const stem = name.slice(0, -".json".length);
    assertRecordPublishable(record, name);
    if (record.key !== stem) {
      throw new Error(`${name}: key ${record.key} does not match filename`);
    }
    hashes[stem] = record.screenshotSha256 as string;
  }
  return hashes;
}

export function assertRecordPublishable(
  record: { key?: string; screenshotSha256?: unknown },
  filename: string,
): void {
  const blob = JSON.stringify(record);
  for (const marker of IMAGE_MARKERS) {
    if (blob.includes(marker)) {
      throw new Error(`${filename}: image payload (${marker}) must not be published`);
    }
  }
  const hash = record.screenshotSha256;
  if (typeof hash !== "string" || !SHA256_HEX.test(hash)) {
    throw new Error(`${filename}: screenshotSha256 must be 64 lowercase hex`);
  }
}

export function assertCatalogMatchesRecords(
  catalog: Record<string, string>,
  records: Record<string, string>,
): void {
  const catalogKeys = Object.keys(catalog).sort();
  const recordKeys = Object.keys(records).sort();
  const onlyCatalog = catalogKeys.filter((k) => records[k] === undefined);
  const onlyRecords = recordKeys.filter((k) => catalog[k] === undefined);
  const mismatched = catalogKeys.filter(
    (k) => records[k] !== undefined && records[k] !== catalog[k],
  );
  if (onlyCatalog.length || onlyRecords.length || mismatched.length) {
    throw new Error(
      `catalog drifted: onlyCatalog=${onlyCatalog.slice(0, 8).join(",")} onlyRecords=${onlyRecords.slice(0, 8).join(",")} mismatched=${mismatched.slice(0, 8).join(",")}`,
    );
  }
}

export function catalogFileJson(hashes: Record<string, string>): string {
  const sorted: Record<string, string> = {};
  for (const key of Object.keys(hashes).sort()) {
    sorted[key] = hashes[key];
  }
  const catalog: ScreenshotCatalog = {
    ruling: "records-only",
    source: RECORDS_REL,
    hashes: sorted,
  };
  return `${canonicalJson(catalog)}\n`;
}

export function loadScreenshotCatalog(repoRoot: string): ScreenshotCatalog {
  const path = join(repoRoot, CATALOG_REL);
  if (!existsSync(path)) {
    throw new Error(
      `${CATALOG_REL} missing; run tsx src/publication.ts --write`,
    );
  }
  const catalog = JSON.parse(readFileSync(path, "utf8")) as ScreenshotCatalog;
  if (catalog.ruling !== "records-only") {
    throw new Error(`${CATALOG_REL}: ruling must be records-only`);
  }
  for (const [key, hash] of Object.entries(catalog.hashes)) {
    if (!SHA256_HEX.test(hash)) {
      throw new Error(`${CATALOG_REL}: ${key} is not 64 lowercase hex`);
    }
  }
  return catalog;
}

export function m1ScreenshotHashes(repoRoot: string): NamedHash[] {
  const file = JSON.parse(
    readFileSync(join(repoRoot, M1_VECTORS_REL), "utf8"),
  ) as {
    vectors: Array<{
      id: string;
      oracle: { screenshot_sha256?: string };
    }>;
  };
  return file.vectors.map((v) => {
    const hash = v.oracle.screenshot_sha256;
    if (typeof hash !== "string" || !SHA256_HEX.test(hash)) {
      throw new Error(`${v.id}: missing screenshot_sha256`);
    }
    return { id: v.id, sha256: hash };
  });
}

export function assertConformanceRecordsHashes(
  markdown: string,
  hashes: NamedHash[],
): void {
  for (const row of hashes) {
    if (!markdown.includes(row.sha256)) {
      throw new Error(
        `CONFORMANCE.md missing screenshot hash for ${row.id}`,
      );
    }
  }
}

export function writeScreenshotCatalog(repoRoot: string): {
  count: number;
  digest: string;
  path: string;
} {
  const hashes = hashesFromRecords(join(repoRoot, RECORDS_REL));
  const path = join(repoRoot, CATALOG_REL);
  writeFileSync(path, catalogFileJson(hashes), "utf8");
  return { count: Object.keys(hashes).length, digest: catalogDigest(hashes), path };
}

export function loadCorpusArchiveMeta(repoRoot: string): CorpusArchiveMeta {
  const path = join(repoRoot, CORPUS_META_REL);
  if (!existsSync(path)) {
    throw new Error(`${CORPUS_META_REL} missing; run scripts/pack-conformance-corpus.sh`);
  }
  const meta = JSON.parse(readFileSync(path, "utf8")) as CorpusArchiveMeta;
  if (meta.filename !== "takehome-conformance-corpus-2026.1.tar.zst") {
    throw new Error(`${CORPUS_META_REL}: unexpected filename ${meta.filename}`);
  }
  if (!SHA256_HEX.test(meta.sha256) || !SHA256_HEX.test(meta.catalog_digest)) {
    throw new Error(`${CORPUS_META_REL}: sha256 fields must be 64 lowercase hex`);
  }
  if (!meta.download.includes(meta.filename)) {
    throw new Error(`${CORPUS_META_REL}: download URL must name the archive`);
  }
  return meta;
}

export function assertArchiveReproducesCatalogDigest(
  archivePath: string,
  expectedDigest: string,
  destDir: string,
): void {
  const unpacked = spawnSync(
    "tar",
    ["--zstd", "-xf", archivePath, "-C", destDir],
    { encoding: "utf8" },
  );
  if (unpacked.status !== 0) {
    throw new Error(
      `tar extract failed (${unpacked.status}): ${unpacked.stderr}`,
    );
  }
  const recordsDir = join(destDir, "records");
  const got = catalogDigest(hashesFromRecords(recordsDir));
  if (got !== expectedDigest) {
    throw new Error(
      `archive catalog digest drifted: ${got} !== ${expectedDigest}`,
    );
  }
}

const isDirect =
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (isDirect && process.argv.includes("--write")) {
  const result = writeScreenshotCatalog(findRepoRoot());
  process.stdout.write(
    `${canonicalJson({ wrote: CATALOG_REL, ...result })}\n`,
  );
}
