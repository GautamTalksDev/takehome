# Contributing

**Who this is for:** Developers opening a pull request against `takehome-ca/takehome`.

**When you finish:** You can run the full check suite, follow tests-first and float rules, and file a bug report we can reproduce.

Changes to payroll math must track T4127 (Payroll Deductions Formulas) and the rule set version selected by each request's **effective date** (`as_of` calendar date).

## Before you open a PR

Run these from the repository root:

```bash
./scripts/float-ban.sh
./scripts/name-ban.sh
./scripts/docs-lint.sh
cargo test --workspace
cd services/api && npm test
```

Fix any failure in the area you touched. Conformance numbers in [`CONFORMANCE.md`](CONFORMANCE.md) regenerate from `tools/conformance`; do not hand-edit agreement rates.

## Tests first

Workspace rule (`.cursor/rules/takehome.mdc`): write or extend the test module before implementation when you add behaviour. If you ask for a feature and no test exists, add the test first and say so in the PR.

Vectors live under `crates/takehome-core/tests/vectors/`. API behaviour lives in numbered files under `services/api/tests/`.

## Float ban

No `f32`, `f64`, or other IEEE-754 binary floats anywhere in `takehome-core`, including tests and comments that show float literals. Construct money and rates from strings. `./scripts/float-ban.sh` enforces this in CI.

## Naming ban

The retired pre-rename product name must not appear in new code or docs outside
allowlisted historical files. `./scripts/name-ban.sh` lists permitted paths
(CHANGELOG, old ADR wording, findings, rename fixtures).

## Documentation house style

Product markdown under `docs/`, root guides, and [`README.md`](README.md) must pass `docs-lint.sh`: opening **Who this is for** / **When you finish**, closing **Last reviewed** and **Engine**, no em dashes, no banned filler words, define T4127 terms on first use per page.

## Add a jurisdiction

1. Read [`docs/jurisdictions.md`](docs/jurisdictions.md). Quebec (`QC`) remains unsupported until QPP and QPIP (Québec Parental Insurance Plan) are implemented end to end.
2. Add provincial rule JSON via ingest from CRA CSV into `data/rules/`.
3. Register the province in `crates/takehome-core/src/jurisdictions.rs` and wire formulas under `crates/takehome-core/src/formulas/province/`.
4. Extend test 28-style transcription checks and grid census if PDOC capture applies.
5. Vendor the new files under `crates/takehome-core/vendor/rules/`.

## Add a rule set version

1. Archive sources in `data/sources/` with hash metadata (`tools/ingest`).
2. Emit `data/rules/<YYYY-MM-DD>/` and run ingest tests.
3. Copy vendored snapshots into `crates/takehome-core/vendor/rules/<YYYY-MM-DD>/`.
4. Register listing status (enacted vs proposed) in the rules registry.
5. Regenerate conformance if PDOC corpora change.

Effective date resolution (which rule set answers an `as_of` date): [`docs/diagrams/02-effective-date-resolution.mmd`](docs/diagrams/02-effective-date-resolution.mmd).

## Good bug report

Include:

- `as_of` effective date, province, `pay_period`, gross and YTD fields, federal and provincial **claim codes** (TD1 claim code indices, not ad hoc dollar amounts unless you intend a custom claim).
- Engine version or `ENGINE_BUILD_SHA256` from the response.
- Expected vs actual cents for each line you care about.
- Whether you compare to PDOC, T4127 text, or another payroll system.

If the bug is a PDOC disagreement, check [`docs/findings/README.md`](docs/findings/README.md) and [`CONFORMANCE.md`](CONFORMANCE.md) first. If the bug is a harness mapping error, label it a **defect**, not a finding.

Architecture overview: [`ARCHITECTURE.md`](ARCHITECTURE.md).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
