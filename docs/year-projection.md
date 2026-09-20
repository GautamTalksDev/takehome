# Year projection

**Who this is for:** Developers calling `POST /v1/deductions/year`.

**When you finish:** You know how YTD carry-forward binds CPP and EI caps, when CPP2 (second additional CPP contribution) starts after YMPE (Year's Maximum Pensionable Earnings), and why the federal tax sum can differ from annual T1 by at most P cents.

`POST /v1/deductions/year` runs every pay period in the year, carrying YTD
CPP, CPP2, EI, and pensionable / insurable earnings forward. Each cheque is
still T4127 (Payroll Deductions Formulas) Option 1 on that period’s gross. The YTD fields are what make
the annual CPP and EI maxima bind, and what delay CPP2 until pensionable
earnings have crossed YMPE (spec §21.5).

## Federal tax sum versus annual T1

The year response includes `annual_t1` (the first period’s Option 1
`annual_projection.federal_tax`, T4127 T1) and `totals.federal_tax` (the sum
of the per-period federal tax lines).

Those two figures are **not required to be equal**. Each period’s federal tax
is `round_tax_to_cent((T1)/P)`: T4127 Chapter 2 / Step 6, per-period
rounding. Adding P independently rounded cheques back up is a different
operation from T1.

The documented tolerance is **one cent per pay period** (`P` cents; `0.26`
on a 26-biweekly year). That is `federal_tax_sum_tolerance` in the JSON.
The gap exists because of per-period rounding, not because the engine used a
different T1.

## Caps and crossings

- `cpp_cap_period` / `ei_cap_period`: 1-based period where the annual
  maximum is reached. Later periods withhold `0.00`. `null` if the year never
  reaches the maximum.
- `ympe_cross_period`: 1-based period where year-to-date pensionable
  earnings first exceed YMPE.
- `cpp2_start_period`: first period with non-zero C2. It is never before
  `ympe_cross_period`.

## 53-week and 27-biweekly years

P = 53 and P = 27 are legal (spec §21.10). The CPP basic exemption is
`truncate_exemption_to_cent(3500 / P)` from T4127 Chapter 6: **$66.03**
weekly-in-a-53-week-year, **$129.62** on a 27-pay biweekly year.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
