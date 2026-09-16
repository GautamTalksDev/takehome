# Finding 001 — K2 maximum in the reaching period

**Record:** M-002 in [`CONFORMANCE.md`](../../CONFORMANCE.md).
**Vector:** `on-biweekly-midyear-k2-max`
**PDOC version:** `2026-06-11` (retrieved 2026-09-12T03:28:07Z)

## T4127 wording

T4127 Chapter 3, factor K2 (glossary note): use the maximum base CPP
contribution “in that pay period.”

The Chapter 4 Step 2 formula is `min(P × C × (0.0495/0.0595), base_max × PM/12)`.
The glossary note is the extra instruction: when year-to-date first reaches the
annual CPP maximum, credit `base_max` in that period, not the still-short
annualized figure.

## PDOC’s observed behaviour

PDOC’s federal CPP credit base is `max(P × C × ratio, D × ratio)` capped at
`base_max × PM/12`. It does **not** force `base_max` in the reaching period.

On the measured vector, `P × C × ratio` is well below `base_max` (this
period’s C is only the remaining room, $30.45), and `D × ratio` is also below
`base_max`. PDOC takes the greater of those two — `D × ratio` — and stops.

## Inversion

Solve T1 from the PDOC period figure, subtract the uncontested EI half, divide
by the lowest rate, compare the resulting CPP credit base against each
candidate.

1. PDOC federal tax `243.95`. Reconstruct annual T1 as `243.95 × 26 = 6342.70`
   (period rounding; the engine’s unrounded T1 under the matching rule is
   `6342.78`, and `round_tax(6342.78 / 26)` is still `243.95`).
2. Every other input to `T3 = (R × A) − K − K1 − K2 − K4` already matched, so
   the residual is K2.
3. The EI half is uncontested: `0.14 × min(P × EI, 1123.07)` with
   `P × 40.75 = 1059.50`. Subtract it and divide by `0.14`.
4. The resulting CPP credit base is **3494.12**, which is
   `D × (0.0495/0.0595)` with `D = 4200.00` (year-to-date *before* this
   period).

Candidates from the same vector:

| Candidate | Value | Match? |
|--|--|--|
| `base_max × PM/12` (maximum fires in this period) | 3519.45 | no |
| raw `P × C × (0.0495/0.0595)` | 658.81 | no |
| `(D × ratio) + (PR × C × ratio)` (YearToDate) | 3519.45 (hits the cap) | no |
| `D × ratio` | **3494.12** | **yes** |

The form that produces that number, and also covers periods after the max is
reached (`D` already at `total_max` → `D × ratio = base_max`), is:

```text
min(base_max × PM/12, max(P × C × ratio, D × ratio))
```

PDOC does not do what the document’s reaching-period sentence says. The
difference is `3519.45 − 3494.12 = 25.33` on the CPP credit base, which is
`$3.54` of annual K2 and **thirteen cents of federal tax in that period**.

## Vector, both outputs, delta

Input (`on-biweekly-midyear-k2-max`):

```json
{
  "as_of": "2026-01-15",
  "province": "ON",
  "pay_period": 26,
  "gross_pay": "2500.00",
  "cpp_months": 12,
  "federal_claim_code": 1,
  "provincial_claim_code": 1,
  "ytd_cpp": "4200.00",
  "ytd_ei": "1000.00",
  "ytd_pensionable_earnings": "71000.00",
  "ytd_insurable_earnings": "50000.00",
  "pay_periods_elapsed": 20
}
```

`ytd_pensionable_earnings` is `71000.00` because PDOC rejects CPP YTD `4200.00`
below its pensionable-earnings consistency floor.

| | PDOC (`pdoc_observed`) | T4127 Chapter 3 reading (`t4127_literal`) | Δ (PDOC − literal) |
|--|--|--|--|
| federal tax | 243.95 | 243.82 | +0.13 |
| provincial tax | 132.29 | 132.24 | +0.05 |
| CPP | 30.45 | 30.45 | 0 |
| EI | 40.75 | 40.75 | 0 |
| total deductions | 447.44 | 447.26 | +0.18 |
| net pay | 2052.56 | 2052.74 | −0.18 |
| K2 (engine) | 637.51 | 641.05 | −3.54 |

PDOC withholds thirteen cents more federal tax, and five cents more
provincial, in the reaching period than T4127’s glossary note. After this
period both paths credit `base_max`; the divergence is this one pay.

YearToDate on the same input is a third figure (federal `243.47`, K2
`649.95`). Test 73 asserts all three K2 values differ.

## Engine default and the alternative

Default `k2_method` is `pdoc_observed`. Customers reconcile against PDOC, so
the oracle wins.

To select the document’s reading:

```json
"k2_method": "t4127_literal"
```

`year_to_date` remains selectable. `annualized` is accepted as an alias of
`pdoc_observed`.

## Reproduce against PDOC (five minutes)

1. Open https://apps.cra-arc.gc.ca/ebci/rhpd/beta/
2. Province of employment: Ontario. Pay date: 2026-01-15. Pay period:
   biweekly (26).
3. Salary or wages: `2500.00`.
4. Enter **Claim codes** 1 (federal) and 1 (provincial). Do not type a fixed
   TD1 dollar amount — a locked `$16,452` skips BPAF phaseout and is a
   different input.
5. Year-to-date: CPP `4200.00`, EI `1000.00`, pensionable earnings
   `71000.00`, insurable earnings `50000.00`, pay periods elapsed `20`.
6. Result screen, PDOC `2026-06-11`: Federal tax deduction **243.95**,
   Provincial **132.29**, CPP **30.45**, EI **40.75**, Total **447.44**,
   Net **2052.56**.

Independent check without the engine: `4200 × 0.0495 / 0.0595 = 3494.1176…`
(PDOC’s CPP credit base) vs `3519.45` (T4127’s `base_max` in this period).
Those two bases, plus the uncontested EI half `1059.50`, times `0.14`,
differ by `$3.54` of annual K2 — thirteen cents once divided by 26 and
rounded.
