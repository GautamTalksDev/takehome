/**
 * Edition provenance for PDOC cache records.
 *
 * `ruleSetVersion` is the case key. `observedEdition` is the calendar edition
 * live PDOC was serving when the result was captured. A record may satisfy a
 * case only when those match, or when data/edition-identity.json proves the
 * jurisdictions touched are identical (test 18 / test 19).
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { PolicyError } from "./pdoc.ts";

export type EditionIdentityPair = {
  a: string;
  b: string;
  identical_jurisdictions: string[];
  identical_claim_code_tables: string[];
  verified_by: string[];
};

export type EditionIdentityFile = {
  description: string;
  pairs: EditionIdentityPair[];
};

export class EditionProvenanceError extends PolicyError {
  constructor(message: string) {
    super(message);
    this.name = "EditionProvenanceError";
  }
}

export function loadEditionIdentity(repoRoot: string): EditionIdentityFile {
  const path = join(repoRoot, "data/edition-identity.json");
  return JSON.parse(readFileSync(path, "utf8")) as EditionIdentityFile;
}

function pairFor(
  identity: EditionIdentityFile,
  left: string,
  right: string,
): EditionIdentityPair | null {
  for (const p of identity.pairs) {
    if ((p.a === left && p.b === right) || (p.a === right && p.b === left)) {
      return p;
    }
  }
  return null;
}

function claimTableForProvince(province: string): string {
  if (province === "OutsideCanada") return "fed";
  return province.toLowerCase();
}

/**
 * Hard error unless the record's observed edition may stand in for the case
 * rule set for every jurisdiction the fingerprint touches (FED + province).
 */
export function assertRecordSatisfiesCase(opts: {
  repoRoot: string;
  observedEdition: string;
  caseRuleSetVersion: string;
  province: string;
}): void {
  const { observedEdition, caseRuleSetVersion, province, repoRoot } = opts;
  if (!observedEdition) {
    throw new EditionProvenanceError(
      "Cache record missing observedEdition. Refuse to satisfy any case.",
    );
  }
  if (observedEdition === caseRuleSetVersion) return;

  const identity = loadEditionIdentity(repoRoot);
  const pair = pairFor(identity, observedEdition, caseRuleSetVersion);
  if (!pair) {
    throw new EditionProvenanceError(
      `No edition-identity pair for observed ${observedEdition} vs case ${caseRuleSetVersion}. Edition-retired / uncapturable.`,
    );
  }

  const neededJurisdictions = ["FED"];
  if (province !== "OutsideCanada") neededJurisdictions.push(province);
  for (const code of neededJurisdictions) {
    if (!pair.identical_jurisdictions.includes(code)) {
      throw new EditionProvenanceError(
        `Record observed under ${observedEdition} cannot satisfy ${caseRuleSetVersion} for ${code}: jurisdiction tables differ (test 18). Edition-retired.`,
      );
    }
  }

  const claimTables = ["fed", claimTableForProvince(province)];
  for (const table of claimTables) {
    if (!pair.identical_claim_code_tables.includes(table)) {
      throw new EditionProvenanceError(
        `Record observed under ${observedEdition} cannot satisfy ${caseRuleSetVersion} for claim-codes/${table}: tables differ (test 19). Edition-retired.`,
      );
    }
  }
}
