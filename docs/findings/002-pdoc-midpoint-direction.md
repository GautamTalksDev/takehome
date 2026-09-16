# Finding 002 — PDOC midpoint direction (open)

**Record:** M-003 in [`CONFORMANCE.md`](../../CONFORMANCE.md).
**Status:** open disagreement. Counts against the PDOC agreement rate.
**PDOC version:** `2026-06-11` (JSON `/SALARY/calculate`).

PDOC does not consistently round exact midpoints the way T4127 Chapter 2
says (third digit ≥ 5 increases the second). On some forms the midpoint
goes **up** (matches the engine). On others it goes **down** (engine +1¢).
The condition that selects the direction is **not named**. Diagnose from
the finished 9,762-form corpus — not from another hand sample.

This is not an Alberta rule. Alberta is where midpoints are dense: 8%
flat on round grosses manufactures exact halves. It is over-represented
in the early queue because the queue is jurisdiction-sorted, not because
the behaviour is provincial.

## Observed classes (all count)

Engine half-up goes up; PDOC’s corresponding line is 1¢ lower.

| Class | Field | Sample | Notes |
|--|--|--|--|
| AB provincial | `T2/P` | 11 forms, P=10 and 12, claim 0 and 1 | Inversion: T2=T4, V1=S=K5P=0. Residual is the period line. |
| AB CPP | period C | 6 forms (`500.00`, `7460.00`, `8500.00` at P=10) | Unrounded C at a 4th-decimal-5 (`8.925`, `423.045`, `484.925`). Provincial exact. |
| NL provincial | `T2/P` | `4333.33` P=12; `1718.39` P=26 | Neighbour `1718.40` P=26 goes **up**. |
| NU provincial | `T2/P` | `2536.41` P=22 | Not NL, not AB. |

Cross-jurisdiction probes (exact engine half-cent `T2/P`, float predicts
engine−1¢): **47/50 unique forms match engine** (up). The three down-cases
are the NL/NU rows above. Float-as-general-PDOC-rule is falsified.

Federal was exact on the AB provincial-down forms — the miss is not
annual income A / F5 on those vectors. PDOC `/SALARY/calculate` has no
annual T2/T4.

## What this is not

| Hypothesis | Result |
|--|--|
| Alberta-specific K5P (rounded vs unrounded K1P+K2P) | **No.** Claim-code forms ⇒ K5P=0. Even claim 10 + max K2P stays under $4,896. |
| 1¢ miss in T4 / V1 / S assembly | **No.** T2=T4, V1=S=0 on the inverted AB provincials. Unrounded T4 produces PDOC on only 2/11. |
| General PDOC IEEE-float `T2/P` | **No.** 47/50 discriminating half-cent forms outside AB match decimal half-up. |
| Two NL rows as a second provincial rule | **No.** Unique NL split is 12/14 up; one NU down-case besides. |

## Open question

**Under what condition does PDOC’s midpoint go down?**

Hint already in the data, not yet tested: NL `1718.39` down vs neighbour
`1718.40` up. Those two differ in the **input**, not in “being at a
midpoint.” If direction depends on something upstream of the rounding
step, the engine’s “exact half-cent” may be exact only in our arithmetic
— a sub-cent tail PDOC still has. That would also explain why unrounded
T4 does not uniquely solve back on 9 of 11 AB provincials.

That is a hypothesis. Pool every midpoint (provincial, CPP, federal)
across the finished corpus and look for what separates down from up.
Do not construct another hand sample while the 9,762-form run is moving.

## Rate discipline

Every live cent delta in the classes above **counts**. M-003 is not
exempted as methodology. M-002 (`k2_method`) had no unavoidable live
delta under the default. Different shape — do not collapse them.

## Product decision

See [`ADR-003`](../ADR-003-rounding-compat.md): `rounding_compat` (`t4127`
default). The `pdoc` arm is a typed `RoundingCompatPdocNotImplemented`
error until this finding names the midpoint condition. A silent alias of
`t4127` is a documented promise the engine does not keep. Do not implement
`pdoc` as IEEE-float residue.
