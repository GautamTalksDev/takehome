import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";

import { findRepoRoot, sha256 } from "./pdoc.ts";
import {
  CORPUS_ARCHIVE_REL,
  SHA256_HEX,
  assertArchiveReproducesCatalogDigest,
  assertCatalogMatchesRecords,
  assertConformanceRecordsHashes,
  assertRecordPublishable,
  catalogDigest,
  catalogFileJson,
  hashesFromRecords,
  loadCorpusArchiveMeta,
  loadScreenshotCatalog,
  m1ScreenshotHashes,
} from "./publication.ts";

const repoRoot = findRepoRoot();

function gitIgnored(rel: string): boolean {
  const result = spawnSync("git", ["check-ignore", "-q", "--", rel], {
    cwd: repoRoot,
  });
  if (result.status === 0) return true;
  if (result.status === 1) return false;
  throw new Error(
    `git check-ignore ${rel} exited ${result.status}: ${result.stderr.toString()}`,
  );
}

describe("screenshot catalog digest", () => {
  it("hashes sorted key-space-sha256 lines", () => {
    assert.equal(
      catalogDigest({}),
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
    const digest = catalogDigest({
      b: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      a: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    });
    const canonical =
      "a aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n" +
      "b bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n";
    assert.equal(digest, sha256(canonical));
  });

  it("moves when a hash is added or dropped", () => {
    const hashes = {
      keep: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      qpip: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    };
    const baseline = catalogDigest(hashes);
    assert.notEqual(
      catalogDigest({ ...hashes, ZZZ_NOT_A_FACTOR: "c".repeat(64) }),
      baseline,
    );
    const dropped = { keep: hashes.keep };
    assert.notEqual(catalogDigest(dropped), baseline);
  });
});

describe("record publication shape", () => {
  it("rejects image payloads and non-hex hashes", () => {
    const ok = {
      key: "abc",
      screenshotSha256: "a".repeat(64),
      output: { net_pay: "1.00" },
    };
    assertRecordPublishable(ok, "abc.json");
    assert.throws(
      () =>
        assertRecordPublishable(
          { ...ok, screenshotBytes: "iVBORw0KGgo=" },
          "abc.json",
        ),
      /image payload/,
    );
    assert.throws(
      () =>
        assertRecordPublishable({ ...ok, screenshotSha256: "not-a-hash" }, "abc.json"),
      /screenshotSha256/,
    );
    assert.throws(
      () =>
        assertRecordPublishable(
          { ...ok, screenshotSha256: null },
          "abc.json",
        ),
      /screenshotSha256/,
    );
  });
});

describe("catalog vs records (live cache)", () => {
  it("every published record has a catalog hash, and the reverse", () => {
    const recordsDir = join(repoRoot, "data/pdoc-cache/records");
    const records = hashesFromRecords(recordsDir);
    const catalog = loadScreenshotCatalog(repoRoot);
    assert.equal(catalog.ruling, "records-only");
    assertCatalogMatchesRecords(catalog.hashes, records);
    const n = Object.keys(records).length;
    assert.ok(n > 0, "records/ is empty; ruling (a) publishes JSON records");
    for (const rec of Object.values(
      // re-read one file to prove SHA256_HEX on a real hash
      { sample: records[Object.keys(records)[0]] },
    )) {
      assert.match(rec, SHA256_HEX);
    }
  });

  it("fails when the catalog is mutated", () => {
    const records = hashesFromRecords(join(repoRoot, "data/pdoc-cache/records"));
    const catalog = { ...loadScreenshotCatalog(repoRoot).hashes };
    catalog.ZZZ_NOT_A_FACTOR = "d".repeat(64);
    assert.throws(
      () => assertCatalogMatchesRecords(catalog, records),
      /catalog drifted/,
    );
    const dropped = { ...loadScreenshotCatalog(repoRoot).hashes };
    delete dropped[Object.keys(dropped).sort()[0]];
    assert.throws(
      () => assertCatalogMatchesRecords(dropped, records),
      /catalog drifted/,
    );
  });

  it("round-trips through catalogFileJson", () => {
    const dir = mkdtempSync(join(tmpdir(), "pdoc-pub-"));
    writeFileSync(
      join(dir, "k.json"),
      `${JSON.stringify({
        key: "k",
        screenshotSha256: "e".repeat(64),
        output: { net_pay: "0.00" },
      })}\n`,
    );
    const hashes = hashesFromRecords(dir);
    const parsed = JSON.parse(catalogFileJson(hashes));
    assert.equal(parsed.ruling, "records-only");
    assertCatalogMatchesRecords(parsed.hashes, hashes);
  });
});

describe("gitignore: catalog in git, records in the release archive", () => {
  it("tracks identity, hash catalog, and archive metadata", () => {
    assert.equal(gitIgnored("data/pdoc-cache/pdoc-identity.json"), false);
    assert.equal(gitIgnored("data/pdoc-cache/screenshot-hashes.json"), false);
    assert.equal(gitIgnored("data/pdoc-cache/corpus-archive.json"), false);
  });

  it("keeps the 9010 records, the archive, screenshots, logs, and checkpoints out of git", () => {
    assert.equal(
      gitIgnored(
        "data/pdoc-cache/records/1afe35e3824169e902fd8ed50f12628ba63f107197ab3957cfc47377fa7bbdb9.json",
      ),
      true,
    );
    assert.equal(
      gitIgnored("data/pdoc-cache/takehome-conformance-corpus-2026.1.tar.zst"),
      true,
    );
    assert.equal(
      gitIgnored("data/pdoc-screenshots/on-weekly-600-cc1.png"),
      true,
    );
    assert.equal(gitIgnored("data/pdoc-cache/records/leak.png"), true);
    assert.equal(gitIgnored("data/pdoc-cache/queue-progress.log"), true);
    assert.equal(gitIgnored("data/pdoc-cache/queue-run.log"), true);
    assert.equal(gitIgnored("data/pdoc-cache/last-run-conditions.json"), true);
    assert.equal(gitIgnored("data/pdoc-cache/queue-checkpoint.json"), true);
    assert.equal(gitIgnored("data/pdoc-cache/smoke-checkpoint.json"), true);
  });

  it("tracked-ban.sh names records/ so 38 MB cannot land in HEAD later", () => {
    const src = readFileSync(join(repoRoot, "scripts/tracked-ban.sh"), "utf8");
    assert.match(src, /data\/pdoc-cache\/records\//);
    assert.match(src, /tar\.zst/);
  });
});

describe("CONFORMANCE.md records the hashes", () => {
  it("contains every M1 screenshot hash and the catalog digest", () => {
    const md = readFileSync(join(repoRoot, "CONFORMANCE.md"), "utf8");
    const m1 = m1ScreenshotHashes(repoRoot);
    assert.equal(m1.length, 20);
    assertConformanceRecordsHashes(md, m1);
    const catalog = loadScreenshotCatalog(repoRoot);
    const digest = catalogDigest(catalog.hashes);
    assertConformanceRecordsHashes(md, [
      { id: "catalog", sha256: digest },
    ]);
    assert.match(md, /records-only/);
    assert.match(md, /data\/pdoc-screenshots/);
    const meta = loadCorpusArchiveMeta(repoRoot);
    assert.match(md, new RegExp(meta.filename.replaceAll(".", "\\.")));
    assert.ok(md.includes(meta.sha256), "CONFORMANCE.md missing archive SHA-256");
    assert.ok(md.includes(meta.download), "CONFORMANCE.md missing archive download URL");
    assert.match(md, /sha256sum -c/);
    assert.match(md, /tar --zstd -xf/);
  });

  it("fails when a hash is stripped from the document", () => {
    const md = readFileSync(join(repoRoot, "CONFORMANCE.md"), "utf8");
    const m1 = m1ScreenshotHashes(repoRoot);
    const stripped = md.replaceAll(m1[0].sha256, "HASH_REMOVED");
    assert.throws(
      () => assertConformanceRecordsHashes(stripped, m1),
      /missing screenshot hash/,
    );
  });
});

describe("local PNGs stay out of git and match M1 hashes when present", () => {
  it("hashes local screenshots against the M1 vectors", () => {
    const m1 = m1ScreenshotHashes(repoRoot);
    const shots = join(repoRoot, "data/pdoc-screenshots");
    let checked = 0;
    for (const row of m1) {
      const path = join(shots, `${row.id}.png`);
      try {
        const bytes = readFileSync(path);
        assert.equal(sha256(bytes), row.sha256, row.id);
        checked += 1;
      } catch (err) {
        if ((err as NodeJS.ErrnoException).code === "ENOENT") continue;
        throw err;
      }
    }
    // Local operator machine has the twenty PNGs; CI clone does not.
    if (checked > 0) assert.equal(checked, 20);
  });
});

describe("conformance corpus archive", () => {
  it("extracts to the catalog digest recorded in CONFORMANCE.md", () => {
    const archive = join(repoRoot, CORPUS_ARCHIVE_REL);
    assert.ok(
      existsSync(archive),
      `${CORPUS_ARCHIVE_REL} missing; run scripts/pack-conformance-corpus.sh`,
    );
    const meta = loadCorpusArchiveMeta(repoRoot);
    const bytes = readFileSync(archive);
    assert.equal(sha256(bytes), meta.sha256, "archive bytes drifted from corpus-archive.json");
    const dest = mkdtempSync(join(tmpdir(), "pdoc-corpus-"));
    assertArchiveReproducesCatalogDigest(archive, meta.catalog_digest, dest);
    const catalog = loadScreenshotCatalog(repoRoot);
    assert.equal(catalogDigest(catalog.hashes), meta.catalog_digest);
  });

  it("fails when the expected digest is not the catalog", () => {
    const archive = join(repoRoot, CORPUS_ARCHIVE_REL);
    assert.ok(existsSync(archive), `${CORPUS_ARCHIVE_REL} missing`);
    const dest = mkdtempSync(join(tmpdir(), "pdoc-corpus-mut-"));
    assert.throws(
      () =>
        assertArchiveReproducesCatalogDigest(archive, "0".repeat(64), dest),
      /archive catalog digest drifted/,
    );
  });
});
