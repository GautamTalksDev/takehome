A **disagreement** is: correctly-specified identical inputs, engine and PDOC
both given exactly what the operator intended, outputs differ. That is a
finding about the world. It is published here with the full input, both
outputs, the delta, and an explanation: including cases where we conclude
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

Until every defined vector in a corpus is measured, this document states
**n**, how many are measured, and how many are pending. It does not compute
an agreement rate on a partial run. A completed corpus states how many match
and lists every disagreement.

### M-001: Published whole-dollar K (ADR-002)

Annual T3/T4 is discontinuous by at most $1.00 at each bracket threshold,
because the CRA publishes the K constants rounded to whole dollars. PDOC
exhibits the same discontinuity; we match it deliberately.

The per-threshold jump equals the difference of that bracket's two K
rounding residuals. Measured 2026 jumps for **all thirteen bracket
tables** (FED plus twelve provinces/territories; both January and July
editions; Outside Canada has no provincial K) live in
`crates/takehome-core/tests/vectors/bracket_discontinuity_2026.json`.

Period-tax vs gross is therefore **not** strictly monotone. The property
`ontario_calculate_invariants` uses the M-001 bound (≤ $0.09 per period
across sampled P ≥ 12). The thirteen-jurisdiction property
`calculate_invariants_all_jurisdictions` scales the same residual
difference by P, adds provincial surtax amplification and the Outside
Canada 48% T1 surtax, and allows two cents of period rounding.
`#[ignore]` is never a resolution for those tests.

Decision:
[`docs/ADR-002-published-constants.md`](docs/ADR-002-published-constants.md).

### M-002: PDOC K2 maximum in the reaching period

T4127 Chapter 3 tells the calculator to use the maximum base CPP contribution
“in that pay period”: the period year-to-date first reaches the annual CPP
maximum. PDOC does not. Its federal CPP credit base is
`max(P × C × ratio, D × ratio)` capped at `base_max`, which on the measured
vector is `D × ratio` = 3494.12 rather than `base_max` = 3519.45.

The period-tax difference is 13¢ federal and 5¢ provincial on
`on-biweekly-midyear-k2-max`. That is a fact about PDOC, not an engine defect:
the default `k2_method` is `pdoc_observed` because customers reconcile against
the oracle. The document’s reading is selectable as `t4127_literal`.

Full write-up (vector, both outputs, delta, inversion, reproduction):
[`docs/findings/001-k2-maximum-in-reaching-period.md`](docs/findings/001-k2-maximum-in-reaching-period.md).

PDOC mapping notes: `employee.federal_tax` is T4127 `[(T1)/P]+L` (PDOC
Federal + Additional). `employee.total_tax` is the sum of the two
separately rounded federal and provincial lines a user can add up on
screen; `breakdown.T` is T4127 Step 6 `round((T1+T2)/P)+L` and may differ
by one cent. Claim code 1 must be entered via PDOC **Claim codes**,
not a fixed TD1 dollar amount: fixed `$16,452` does not phase out BPAF.
`employee.total_deductions` includes union dues.

### M-003: PDOC midpoint direction (open)

Live cent delta: T4127 half-up at an exact midpoint goes up; PDOC’s line is
sometimes 1¢ lower. Counts against the agreement rate: not exempted as
methodology. Not Alberta-specific; AB is dense because 8% flat on round
grosses manufactures halves. Observed down-classes: AB provincial `T2/P`,
AB CPP, NL `4333.33` P=12, NL `1718.39` P=26 (neighbour `1718.40` goes up),
NU `2536.41` P=22. Cross-jurisdiction probes: **47/50** discriminating
half-cent forms match engine (up). Condition that selects down vs up is unnamed. Finished July queue: **164**
one-cent disagreements across thirteen jurisdictions (AB 40, ON 18, BC 14,
NU 14, NS 12, MB 11, YT 11, NL 10, PE 8, SK 8, NB 6, NT 6, OutsideCanada 6);
**780** forms the salary UI accepted as queue items but PDOC step-2 would
not advance. Those 780 are named, not pending, and not in the rate.

Product: [`ADR-003`](docs/ADR-003-rounding-compat.md): `rounding_compat`
(`t4127` default; `pdoc` is a typed not-implemented error until finding 002).

Full write-up:
[`docs/findings/002-pdoc-midpoint-direction.md`](docs/findings/002-pdoc-midpoint-direction.md).

### M4 named classes (not folded into 8838/9002)

- **`pdoc-bonus-2026`**: 20 PDOC bonus vectors captured 2026-09-19 through `tools/pdoc-oracle` (locator: Total current bonus payable). Own class; excluded from the overall Option 1 PDOC rate. One-cent disagreements listed on the corpus; expected amounts stay PDOC.
- **`year-projection-2026`**: engine invariants (API tests 22-29). Oracle class `invariants`. Caps, YMPE/CPP2, BC January/July split, federal sum vs T1 within P cents, 53-week / 27-biweekly exemptions.
- **`t4127-option2-2026`**: T4127 Chapter 5 worked examples plus invariants. Live PDOC has no Option 2 control. Weaker evidence than PDOC; does not inherit the Option 1 agreement rate.

### PDOC evidence publication (ruling a)

JSON cache records are published as the GitHub release archive
`takehome-conformance-corpus-2026.1.tar.zst`, not in git. `pdoc-identity.json`
and `screenshot-hashes.json` stay in the repo so the catalog digest is
checkable without a 38 MB clone. PNG screenshots stay local
(`data/pdoc-screenshots/`, gitignored). This document records the catalog
digest, the archive SHA-256, and the twenty M1 screenshot hashes. Publication
tests extract the archive and break the catalog on purpose.
