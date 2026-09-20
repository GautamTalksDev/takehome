# Request fields

**Who this is for:** Integrators building JSON bodies for `POST /v1/deductions`, batch pay runs, or year projection.

**When you finish:** You know which T4127 (Payroll Deductions Formulas) inputs are required, their defaults, and when to send each optional field.

A **claim code** is the TD1 box value that maps to federal and provincial personal amounts. **Proration** applies when Option 1 splits mid-year provincial rates (see [effective-dates.md](./effective-dates.md)).

Money is always a quoted decimal string with two fractional digits (for example `"1000.00"`). JSON numbers are rejected.

## Six fields most callers need

| Field | Purpose |
| --- | --- |
| `as_of` | **Effective date** (calendar date) that selects which enacted rule set applies. |
| `province` | Two-letter jurisdiction code (`ON`, `BC`, `AB`, …). |
| `pay_period` | Pay periods per year (`52` weekly, `26` biweekly, …). |
| `gross_pay` | Taxable gross for this cheque. |
| `federal_claim_code` | Federal TD1 claim code (integer 0 to 10 or letter codes where supported). |
| `provincial_claim_code` | Provincial TD1 claim code. |

Send both claim codes unless you supply explicit `federal_tc` / `provincial_tcp` amounts instead (never both code and amount).

## Identity and dates

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `as_of` | date | Worker UTC date if omitted | Recommended always | Selects tables for `R`, `K`, `V`, YMPE (Year's Maximum Pensionable Earnings), BPAF (Basic Personal Amount Factor) | Every production call; omit only when you accept the server clock |
| `province` | string | none | Yes | Table 8.1 jurisdiction | Always |
| `pay_period` | integer | none | Yes | `P`, `PR` | Always; must be a legal T4127 period count |
| `prior_province` | string | none | No | Provincial switch mid-year | Employee moved provinces during the tax year |
| `date_of_birth` | date | none | No | Age-based credits or exemptions | Rare; CPP election scenarios |
| `calculation_option` | string | `option1` | No | Option 1 vs Option 2 annualization | BC, NL, PE mid-year **proration** paths; see [effective-dates.md](./effective-dates.md) |
| `cpp_months` | integer | `12` | No | CPP contribution months | Short-year employment |
| `k2_method` | string | `pdoc_observed` | No | `K2` / `K2P` base | PDOC reconciliation vs literal T4127 |
| `rounding_compat` | string | `t4127` | No | Rounding steps | Legacy PDOC rounding only when needed |

## Income

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `gross_pay` | money | none | Yes | Feeds `IE`, annual `A` | Always |
| `pensionable_earnings` | money | `gross_pay` | No | CPP pensionable | Differs from gross (benefits, caps) |
| `insurable_earnings` | money | `gross_pay` | No | EI insurable | Differs from gross |
| `taxable_benefits` | money | `0.00` | No | Taxable allowances | Non-cash taxable items in the period |
| `retroactive_pay` | money | none | No | Retroactive lump | T4127 retro pay instructions |

## Claims and personal amounts

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `federal_claim_code` | int/string | none | One of code or TC | `TC`, `K1` | Default withholding setup |
| `provincial_claim_code` | int/string | none | One of code or TCP | `TCP`, `K1P` | Default withholding setup |
| `federal_tc` | money | none | Mutually exclusive with code | `TC` | Custom federal total claim amount |
| `provincial_tcp` | money | none | Mutually exclusive with code | `TCP` | Custom provincial total claim amount |
| `dependants_under_19` | integer | none | No | Provincial credits | MB and similar |
| `dependants_disabled` | integer | none | No | Provincial credits | Where TD1X applies |

## Deductions from pay

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `rpp` | money | none | No | Registered pension | Period RRSP/RPP |
| `union_dues` | money | none | No | Union dues credit | Period dues |
| `alimony` | money | none | No | Support payments | Ordered deductions |
| `child_care_expenses` | money | none | No | Child care | Provincial rules |
| `prescribed_zone_deduction` | money | none | No | Northern deduction | Zone allowances |
| `lcf_purchase` | money | none | No | `LCF` | Labour-sponsored funds |
| `lcp_purchase` | money | none | No | `LCP` | Provincial labour credit |
| `additional_tax_requested` | money | none | No | `L` extra tax | TD1 “additional tax” box |
| `cpp_exempt` | boolean | false | No | CPP exemption | Valid exemption on file |
| `cpp_election_after_65` | boolean | false | No | CPP after 65 | Employee election |

## Year-to-date (YTD)

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `ytd_pensionable_earnings` | money | none | No | CPP YTD PE | Mid-year hire or CPP cap |
| `ytd_insurable_earnings` | money | none | No | EI YTD | EI cap tracking |
| `ytd_cpp` | money | none | No | `D` | CPP paid YTD |
| `ytd_cpp2` | money | none | No | `D2`, CPP2 (second additional CPP) | YAMPE (Year's Additional Maximum Pensionable Earnings) band |
| `ytd_ei` | money | none | No | EI premiums YTD | EI cap |
| `ytd_federal_tax` | money | none | No | Federal tax paid | Bonus methods |
| `ytd_provincial_tax` | money | none | No | Provincial tax paid | Bonus methods |
| `ytd_income` | money | none | No | Cumulative income | Commission averaging |
| `ytd_rpp` | money | none | No | RPP YTD | Annualized RPP |
| `ytd_union_dues` | money | none | No | Dues YTD | Credits |
| `f5a_ytd` | money | none | No | `F5A` YTD | CPP additional |
| `f5b_ytd` | money | none | No | `F5B` YTD | CPP additional |
| `pay_periods_elapsed` | integer | none | No | Period index | Mid-year start |
| `ytd_qpp` / `ytd_qpp2` / `ytd_qpip` | money | none | No | Quebec fields | Reserved; QC not supported |

## Bonus and lump sums

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `bonus` | money | none | No | Bonus withholding | T4127 bonus method |
| `bonus_method` | string | `regular` | No | Method selector | Alternate bonus treatment |
| `ytd_bonus` | money | none | No | YTD bonus | Cumulative bonus |
| `bonus_rrsp` | money | none | No | Bonus RRSP | Special RRSP bonus |
| `ytd_bonus_rrsp` | money | none | No | YTD bonus RRSP | Tracking |
| `most_recent_i` | money | none | No | Recent insurable | Bonus averaging |

## Option 2 and commission

| Field | Type | Default | Required | T4127 factor | When needed |
| --- | --- | --- | --- | --- | --- |
| `calculation_option` | string | `option1` | No | Option 1 vs 2 | Set `option2` when your software uses T4127 Option 2 |
| `commission_income` | money | none | No | Commission `A` | Commission employees |
| `commission_expenses` | money | none | No | Expenses | Net commission |
| `estimated_annual_expenses` | money | none | No | Expense estimate | Annual expense election |

Unknown property names yield `unknown_field` (HTTP 400). The API strips no extra keys silently.

See [api-reference.md](./api-reference.md) for routes and [errors.md](./errors.md) for rejection behavior.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
