# Finding 002: PDOC midpoint direction (open)

**Who this is for:** Engineers investigating the open one-cent PDOC disagreements (M-003).

**When you finish:** You know M-003 stays open, what classes exist, and how to reproduce a case.


**Record:** M-003 in [`CONFORMANCE.md`](../../CONFORMANCE.md).
**Status:** open disagreement. Counts against the PDOC agreement rate.
**Corpus:** July capturable queue, finished. 8,818 match / 164 disagree /
780 step-2 non-advance (8,982 + 780 = 9,762). Tag `v0.3.0-m2`.

Every one of the 164 live deltas is **±1¢**. They are not all the same
class, and they are not all “PDOC rounded the midpoint down.”

## Field (where in the pipeline)

| Tax line in the delta | n | Engine − PDOC |
|--|--|--|
| CPP only | 81 | always **+1¢** CPP (PDOC lower) |
| Provincial only | 63 | **mixed**: 45 engine higher, 18 engine lower |
| Federal only | 18 | **mixed**: 15 engine higher, 3 engine lower |
| CPP + provincial | 1 | NB `7460.00` P=10 claim 1: CPP +1¢, provincial −1¢ |
| Federal + provincial | 1 | ON `620.51` P=12 claim 0: federal −1¢, provincial +1¢ |

CPP is one finding: PDOC’s period C is 1¢ below T4127 (Payroll Deductions Formulas) half-up, 82 times
including the mixed NB row. Provincial and federal are not that clean.
Alberta provincials are uniformly engine-higher (32). Ontario provincials
are mostly engine-lower (7 of 8). A single “always down” rule is false.

## Are they midpoints?

On the **engine’s already-cent T2**, `T2/P` is an exact half-cent for
**34 of 65** provincial disagreements and not for **31 of 65**.
`T1/P` is an exact half-cent for **9 of 19** federal disagreements.

So: not all 164 are midpoints in our arithmetic. A second class exists : 
1¢ misses where the engine period line is not on a half-cent. Implementing
`rounding_compat: pdoc` as “IEEE-float at the half” would not zero the
164, and would mis-handle the 19 provincial cases where PDOC is *higher*.

CPP midpoints are not visible from already-rounded period C; the 82 CPP
rows remain consistent with a 4th-decimal-5 on unrounded C (the six AB
examples still stand) but that is not proven for all 82.

## Direction

Both behaviours exist, in the published corpus, not only in the old 50-form
probe (47/50 engine-up at discriminating half-cents).

- **PDOC lower** (engine tax/CPP +1¢): all 82 CPP; 45 provincial-only; 15 federal-only.
- **PDOC higher** (engine tax −1¢): 18 provincial-only; 3 federal-only.

The discriminator is not “midpoint ⇒ down.”

## Neighbour pairs (tested at scale)

Every one of the 164 has a same-province / same-P / same-claims neighbour
**1¢ of gross away**. **159** of those neighbours are **not** in the 164
(they compared equal). Five disagreements sit next to another disagreement.

The parked NL pair survives and is the type specimen:

| Gross | In the 164? | PDOC provincial | PDOC CPP |
|--|--|--|--|
| `1718.37` | no | 138.86 | 94.23 |
| `1718.38` | no | 138.86 | 94.23 |
| `1718.39` | **yes** | 138.86 | 94.23 |
| `1718.40` | no | 138.87 | 94.24 |

Engine provincial on `1718.39` is 138.87. PDOC holds 138.86 for three
consecutive cents of gross and ticks on `1718.40`. Direction tracks the
**input step**, not “this form’s T2/P is a midpoint.” That is the
upstream-tail hypothesis, now with 164/164 one-cent neighbours.

Alberta `11704.49` (disagree) vs `11704.50` (not in the 164) is the same
shape on the 8% flat.

## What this is not (still)

| Hypothesis | Result |
|--|--|
| Alberta-specific K5P | **No.** All 13 jurisdictions appear. AB is dense, not unique. |
| General PDOC IEEE-float `T2/P` | **No.** 31 provincial misses are not engine T2/P half-cents; 19 provincials have PDOC *higher*. |
| One named midpoint direction | **No.** Up and down both occur. |
| A `rounding_compat: pdoc` arm we can ship | **Not yet.** No discriminator that turns 164 into 0 without a silent float alias. |

M4 (tag `v0.5.0-m4`) re-ran the 164-case corpus as an integer-cent probe.
T2/P half-cent provincials are uniformly engine-higher, so a half-cent-down
arm on that subset would not fight mixed direction there: but it would
still leave the 81 CPP-only rows, the 31 provincial misses that are not
engine T2/P half-cents, and the mixed-direction tax rows that live off-half.
No discriminator. `rounding_compat: pdoc` remains
`RequestError::RoundingCompatPdocNotImplemented`.

## Open question

**What holds PDOC’s displayed line one gross-cent later (or earlier) than
T4127 half-up?** The neighbour evidence says the split is upstream of the
rounding step we see. Engine “exact half-cent” is exact in our cents; PDOC
may still be carrying a sub-cent tail. That is still unnamed.

## Rate discipline

All 164 count. M-003 is not exempted as methodology. Do not implement
`rounding_compat: pdoc` as IEEE-float residue
([ADR-003](../ADR-003-rounding-compat.md)).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
