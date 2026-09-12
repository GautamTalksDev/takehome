# Conformance

Oracle is CRA PDOC (`https://apps.cra-arc.gc.ca/ebci/rhpd/beta/`).
PDOC version observed on filled vectors: `2026-06-11`.
Capture operator: manual (Cursor browser).

Expected amounts are PDOC-only. They are never filled from the engine.

## Methodology

A **disagreement** is: correctly-specified identical inputs, engine and PDOC
both given exactly what the operator intended, outputs differ. That is a
finding about the world. It is published here with the full input, both
outputs, the delta, and an explanation — including cases where we conclude
PDOC is wrong.

A disagreement is **not**: a bug in the harness, a mis-mapped request or
result field, or a vector whose input was specified differently on the two
sides. That is a **defect**. Fix it, add a regression test, and it never
appears in this record.

The distinction is the document’s credibility. Inflating the disagreement
count with our own mapping bugs is less trustworthy, not more honest.
Publishing everything is not the same as publishing indiscriminately. The
number of disagreements below means genuine remaining deltas, not “every
time a test failed.”

Until every defined vector is measured, this document states **n**, how many
are measured, and how many are pending. It does not compute an agreement
rate on a partial run. With all twenty measured, it states how many match
and lists every disagreement.

### M-001 — Published whole-dollar K (ADR-002)

Annual federal tax is discontinuous by up to $1.00 at each bracket threshold,
because the CRA publishes the K constants rounded to whole dollars. PDOC
exhibits the same discontinuity; we match it deliberately.

Period-tax vs gross is therefore **not** strictly monotone. Measured adjacent
cent drops are 1–2¢. The property `ontario_calculate_invariants` uses the
M-001 bound (≤ $0.09 per period across legal P), not a one-cent continuity
claim. `#[ignore]` is never a resolution for that test.

Measured 2026 jumps live in
`crates/netpay-core/tests/vectors/bracket_discontinuity_2026.json`.
Decision:
[`docs/ADR-002-published-constants.md`](docs/ADR-002-published-constants.md).

PDOC mapping notes: `employee.federal_tax` is T4127 `[(T1)/P]+L` (PDOC
Federal + Additional). Claim code 1 must be entered via PDOC **Claim codes**,
not a fixed TD1 dollar amount — fixed `$16,452` does not phase out BPAF.
`employee.total_deductions` includes union dues.

## Status

| | |
|--|--|
| Cases defined (Ontario 2026-01) | 20 |
| Measured | 20 |
| Pending | 0 |
| Exact matches | 20 |
| Open disagreements | 0 |
| Open defects (measured set) | 0 |

M1 tag `v0.2.0-m1` cut 2026-09-12 after D-007 closed by inversion.

### M1 exit checklist

| Criterion | State |
|--|--|
| §7a rulings applied | yes |
| §7b transcription dated in `manifest.json` | `scalars_verified_at` recorded |
| robots.txt checked and recorded | yes — PDOC path allowed (canada.ca hash `5e6c293b…`; apps 404=no file) |
| 20/20 vectors captured with full provenance | **yes** |
| Zero open defects; remaining deltas are genuine disagreements | **yes** (zero open disagreements) |
| Methodology note from §8 | this file |
| Bracket discontinuity fixture + ADR-002 | yes |
| `ontario_calculate_invariants` un-skipped, M-001-bounded | **passes** |
| Tagged `v0.2.0-m1` | **yes** |

## Measured results

| Vector | Result |
|--|--|
| `on-weekly-600-cc1` | match |
| `on-weekly-1000-cc1` | match |
| `on-weekly-2000-cc1` | match |
| `on-weekly-above-ympe` | match |
| `on-weekly-above-yampe` | match |
| `on-weekly-bpaf-phaseout` | match |
| `on-weekly-surtax-tier1` | match |
| `on-weekly-surtax-tier2` | match |
| `on-v2-tier0-at-or-below-20k` | match |
| `on-v2-tier1-20k-36k` | match |
| `on-v2-tier2-36k-48k` | match |
| `on-v2-tier3-48k-72k` | match |
| `on-v2-tier4-72k-200k` | match |
| `on-v2-tier5-above-200k` | match |
| `on-weekly-claim-code-e` | match |
| `on-weekly-claim-code-0` | match |
| `on-weekly-direct-tc-tcp` | match |
| `on-monthly-1000-cc1` | match |
| `on-weekly-l-and-u1` | match |
| `on-biweekly-midyear-k2-max` | match (D-007 closed) |

## Disagreements

None open.

### Closed — D-007 biweekly mid-year K2

**Vector:** `on-biweekly-midyear-k2-max` (full input / both outputs / Δ documented
while open; now matches).

**Inversion (2026-09-12):** From PDOC federal `243.95` → `T1 = 6342.70` →
`K2_pdoc = K2_ours − 3.46`. EI half uncontested → PDOC CPP credit base
**3494.12** = `D × (0.0495/0.0595)` with `D = 4200` (before this period).

Four named candidates: only the residual (~24.7 = this period’s `C × ratio`)
identified the fifth form. PDOC does **not** force `base_max` in the period
YTD first reaches the annual CPP max.

**Engine change:** Annualized K2 CPP base =
`min(base_max×PM/12, max(P×C×ratio, D×ratio))`. Default remains Annualized;
YearToDate stays selectable. Vector now matches to the cent (incl. provincial).

## Closed defects (not disagreements)
## Closed defects (not disagreements)

- **CPP product-once rounding** (was D-004).
- **L / U1 field mapping** (was D-005).
- **CPP2 annualized band** (was D-003 / D-006): literal
  `W = greater of PI_YTD and (YMPE × PM/12)`; C2 is year-to-date. Vector 5
  uses `ytd_pensionable ≈ 84000` so the remaining-room cap binds (`C2 = 16.00`).
- **Federal credits used bracket R**: K1/K2/K4 use lowest federal rate `0.1400`.
- **BPAF capture via fixed TD1 dollars**: high-income vectors must use PDOC
  Claim codes so BPAF phaseout applies; fixed `16,452.00` does not.

## Coverage gaps

- Provinces other than Ontario (M2+).
- Option 2 cumulative averaging.
- Quebec / QPIP end-to-end.

## M0 arithmetic canaries

- `truncate_exemption_to_cent(3500/12) == 291.66`
- Twin: `round_contribution_to_cent` on the same input is `291.67`
- `./scripts/float-ban.sh` clean
