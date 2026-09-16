import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import {
  assertRecordSatisfiesCase,
  EditionProvenanceError,
  loadEditionIdentity,
} from "./edition.ts";
import { findRepoRoot } from "./pdoc.ts";

const here = dirname(fileURLToPath(import.meta.url));

describe("edition provenance", () => {
  it("loads edition-identity.json from the repo", () => {
    const root = findRepoRoot(here);
    const id = loadEditionIdentity(root);
    assert.equal(id.pairs.length, 1);
    assert.ok(id.pairs[0]!.identical_jurisdictions.includes("ON"));
    assert.ok(!id.pairs[0]!.identical_jurisdictions.includes("BC"));
  });

  it("allows July-observed records for January ON cases", () => {
    const root = findRepoRoot(here);
    assertRecordSatisfiesCase({
      repoRoot: root,
      observedEdition: "2026-07-01",
      caseRuleSetVersion: "2026-01-01",
      province: "ON",
    });
  });

  it("rejects July-observed records for January BC cases", () => {
    const root = findRepoRoot(here);
    assert.throws(
      () =>
        assertRecordSatisfiesCase({
          repoRoot: root,
          observedEdition: "2026-07-01",
          caseRuleSetVersion: "2026-01-01",
          province: "BC",
        }),
      EditionProvenanceError,
    );
  });

  it("rejects July-observed records for January NL claim-code cases", () => {
    const root = findRepoRoot(here);
    assert.throws(
      () =>
        assertRecordSatisfiesCase({
          repoRoot: root,
          observedEdition: "2026-07-01",
          caseRuleSetVersion: "2026-01-01",
          province: "NL",
        }),
      EditionProvenanceError,
    );
  });
});
