const PROBE =
  '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}';

let cachedVersion;

export async function engineVersion(engine) {
  if (cachedVersion) {
    return cachedVersion;
  }
  const parsed = JSON.parse(await Promise.resolve(engine.calculate(PROBE)));
  if (!parsed.engine_version) {
    throw new Error('probe calculate did not return engine_version');
  }
  cachedVersion = parsed.engine_version;
  return cachedVersion;
}

export async function liveIdentity(engine, ruleSetVersion = null) {
  return {
    rule_set_version: ruleSetVersion,
    engine_version: await engineVersion(engine),
    engine_build_sha256: await Promise.resolve(engine.engineBuildSha()),
  };
}

export async function latestRuleSetVersion(engine) {
  const listing = JSON.parse(
    await Promise.resolve(engine.listRuleSetVersions()),
  );
  const rows = listing.rule_set_versions;
  return rows[rows.length - 1].version;
}
