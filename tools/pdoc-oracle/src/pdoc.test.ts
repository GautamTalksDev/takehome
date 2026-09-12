import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

import {
  assertIdentityMatchesCache,
  backoffMs,
  cacheKey,
  canonicalJson,
  fingerprintFormStructure,
  identityFromPage,
  originAllowsPdoc,
  parseRobots,
  pathAllowed,
  PdocIdentityDriftError,
  PolicyError,
  requireOpsDoc,
  rulesForAgent,
  sha256,
  USER_AGENT,
  type RobotsFetch,
} from "./pdoc.ts";

const fixtures = join(fileURLToPath(new URL("../fixtures", import.meta.url)));

describe("robots.txt parser", () => {
  it("treats empty Disallow as unrestricted", () => {
    const groups = parseRobots("User-agent: *\nDisallow:\n");
    const rules = rulesForAgent(groups, USER_AGENT);
    assert.equal(pathAllowed(rules, "/ebci/rhpd/beta/entry"), true);
  });

  it("stops on an explicit PDOC disallow", () => {
    const groups = parseRobots(
      "User-agent: *\nDisallow: /ebci/rhpd/\nAllow: /ebci/rhpd/beta/help\n",
    );
    const rules = rulesForAgent(groups, USER_AGENT);
    assert.equal(pathAllowed(rules, "/ebci/rhpd/beta/entry"), false);
    assert.equal(pathAllowed(rules, "/ebci/rhpd/beta/help"), true);
  });

  it("lets a more specific user-agent group win", () => {
    const groups = parseRobots(
      "User-agent: *\nDisallow: /\n\nUser-agent: Netpay-PDOC-Oracle\nDisallow:\n",
    );
    const rules = rulesForAgent(groups, USER_AGENT);
    assert.equal(pathAllowed(rules, "/ebci/rhpd/beta/entry"), true);
  });

  it("honours longest match including * globs", () => {
    const groups = parseRobots(
      "User-agent: *\nDisallow: /en/*/search.html\nDisallow: /content/dam/cra-arc/formspubs/\n",
    );
    const rules = rulesForAgent(groups, USER_AGENT);
    assert.equal(pathAllowed(rules, "/en/foo/search.html"), false);
    assert.equal(
      pathAllowed(
        rules,
        "/en/revenue-agency/services/e-services/e-services-businesses/payroll-deductions-online-calculator.html",
      ),
      true,
    );
  });

  it("canada.ca fixture does not disallow PDOC-on-canada.ca", () => {
    const body = readFileSync(join(fixtures, "canada.ca-robots.txt"), "utf8");
    assert.equal(
      sha256(body),
      "5e6c293b04d4b15808595bf89bb179003f0e74ead9ca5219ec25944a25d78826",
    );
    const fetch: RobotsFetch = {
      url: "https://www.canada.ca/robots.txt",
      finalUrl: "https://www.canada.ca/robots.txt",
      status: 200,
      body,
      sha256: sha256(body),
      fetchedAt: "2026-09-12T02:51:12Z",
      present: true,
    };
    assert.equal(originAllowsPdoc(fetch, "https://www.canada.ca"), true);
  });

  it("treats a 404 HTML body as no robots.txt (allow)", () => {
    const fetch: RobotsFetch = {
      url: "https://apps.cra-arc.gc.ca/robots.txt",
      finalUrl: "https://apps.cra-arc.gc.ca/robots.txt",
      status: 404,
      body: "<!DOCTYPE html><html><title>File not found</title></html>",
      sha256: sha256("<!DOCTYPE html><html><title>File not found</title></html>"),
      fetchedAt: "2026-09-12T02:51:15Z",
      present: false,
    };
    assert.equal(originAllowsPdoc(fetch, "https://apps.cra-arc.gc.ca"), true);
  });
});

describe("cache key and identity", () => {
  it("is stable under key reordering", () => {
    const a = cacheKey({ b: "2", a: "1" }, "2026-01-01");
    const b = cacheKey({ a: "1", b: "2" }, "2026-01-01");
    assert.equal(a, b);
    assert.notEqual(a, cacheKey({ a: "1", b: "2" }, "2026-07-01"));
  });

  it("prefers a version string over the form fingerprint", () => {
    assert.equal(
      identityFromPage({
        versionString: "2026-06-11",
        formFingerprint: "abc",
      }),
      "2026-06-11",
    );
    assert.equal(
      identityFromPage({ versionString: null, formFingerprint: "abc" }),
      "form:abc",
    );
  });

  it("form fingerprint is order-independent", () => {
    const a = fingerprintFormStructure([
      { name: "prov", type: "select", label: "Province" },
      { name: "pay", type: "select", label: "Pay period" },
    ]);
    const b = fingerprintFormStructure([
      { name: "pay", type: "select", label: "Pay period" },
      { name: "prov", type: "select", label: "Province" },
    ]);
    assert.equal(a, b);
  });

  it("alarms when observed identity differs from the cache", () => {
    assert.throws(
      () =>
        assertIdentityMatchesCache(
          {
            pdocIdentity: "2026-06-11",
            recordedAt: "2026-09-12T00:00:00Z",
            source: "pdoc-entry-probe",
          },
          "2026-07-01",
        ),
      PdocIdentityDriftError,
    );
    assertIdentityMatchesCache(null, "2026-06-11");
    assertIdentityMatchesCache(
      {
        pdocIdentity: "2026-06-11",
        recordedAt: "2026-09-12T00:00:00Z",
        source: "pdoc-entry-probe",
      },
      "2026-06-11",
    );
  });

  it("canonicalJson is byte-stable", () => {
    const a = canonicalJson({ z: 1, a: [2, { k: "3" }] });
    const b = canonicalJson({ a: [2, { k: "3" }], z: 1 });
    assert.equal(a, b);
  });
});

describe("policy", () => {
  it("fail-closed without the operations doc", async () => {
    const dir = await mkdtemp(join(tmpdir(), "netpay-pdoc-"));
    assert.throws(() => requireOpsDoc(dir), PolicyError);
  });

  it("backoff is 3s, 6s, 12s, capped at 180s", () => {
    assert.equal(backoffMs(1), 3_000);
    assert.equal(backoffMs(2), 6_000);
    assert.equal(backoffMs(3), 12_000);
    assert.equal(backoffMs(10), 180_000);
  });
});
