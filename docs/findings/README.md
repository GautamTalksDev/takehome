# Findings index

**Who this is for:** Maintainers tracing published PDOC disagreements back to named methodology records.

**When you finish:** You know which findings are closed, which stay open, and where to reproduce each one.

Findings are numbered write-ups linked from [`CONFORMANCE.md`](../../CONFORMANCE.md) methodology sections. They reference T4127 (Payroll Deductions Formulas) readings and live PDOC behaviour. They are not excuses to drop cent deltas from the agreement rate unless the methodology section says otherwise.

## Superseded filename

[`002-alberta-half-cent-float.md`](002-alberta-half-cent-float.md) is retired. IEEE-float as a general PDOC rule is falsified, and the behaviour is not Alberta-specific. The live record is [`002-pdoc-midpoint-direction.md`](002-pdoc-midpoint-direction.md).

## M-001: Published whole-dollar K

- **Record:** M-001 in CONFORMANCE (not a separate finding page)
- **ADR:** [`ADR-002-published-constants.md`](../ADR-002-published-constants.md)

**Known:** CRA publishes bracket constant K rounded to whole dollars. Annual tax can jump by up to one dollar at a threshold. PDOC shows the same discontinuity. Tests bound period withholding jumps.

**Unknown:** Nothing open for M-001. The jump sizes for all thirteen 2026 bracket tables live in `crates/takehome-core/tests/vectors/bracket_discontinuity_2026.json`.

**Reproduce:** Run `cargo test -p takehome-core bracket_discontinuity` and read CONFORMANCE § M-001.

## M-002: K2 maximum in reaching period

- **Record:** M-002
- **Page:** [`001-k2-maximum-in-reaching-period.md`](001-k2-maximum-in-reaching-period.md)
- **Vector:** `on-biweekly-midyear-k2-max`

**Known:** T4127 Chapter 3 asks for maximum base CPP contribution in the pay period when year-to-date first reaches the annual maximum. PDOC uses `min(base_max, max(P×C×ratio, D×ratio))` without forcing `base_max` in that period. Inversion on the vector yields CPP credit base `3494.12` (PDOC) vs `3519.45` (literal reading). Period federal tax differs by thirteen cents under default `pdoc_observed`.

**Unknown:** Whether CRA will align PDOC with the glossary sentence. Customers can select `k2_method: t4127_literal` for the document reading.

**Reproduce:** Five-minute PDOC steps in [finding 001](001-k2-maximum-in-reaching-period.md#reproduce-against-pdoc-five-minutes). Engine: same JSON in `crates/takehome-core/tests/vectors/` PDOC Ontario corpus.

## M-003: PDOC midpoint direction (OPEN)

- **Record:** M-003
- **Status:** **OPEN**
- **Page:** [`002-pdoc-midpoint-direction.md`](002-pdoc-midpoint-direction.md)
- **ADR:** [`ADR-003-rounding-compat.md`](../ADR-003-rounding-compat.md)

**Known:** Finished July capturable queue: 8,818 match, **164** one-cent disagreements (all count in the rate), 780 step-2 non-advances (named, not in the rate). Splits: 82 CPP-only (engine higher), provincial and federal mixes include PDOC higher and lower. Not all 164 are exact half-cent lines in engine arithmetic. `rounding_compat: pdoc` returns `RoundingCompatPdocNotImplemented`.

**Unknown:** The condition that selects PDOC down versus up on a given line. Neighbour gross pairs (164/164 have a ±1¢ neighbour; 159 neighbours match) suggest an upstream tail, not a single midpoint rule.

**Reproduce:** Pick any id from the M-003 rows in [`CONFORMANCE.md`](../../CONFORMANCE.md) disagreements table (for example `NL-P26-1718.39-F0-P0`). Enter the grid form in live PDOC for the July edition. Compare provincial and CPP period lines to `/v1/deductions` with the same `as_of`, province, `pay_period`, gross, and federal and provincial claim codes (TD1 claim code indices). See [finding 002](002-pdoc-midpoint-direction.md) for the NL type specimen table.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
