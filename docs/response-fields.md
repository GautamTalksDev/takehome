# Response fields

**Who this is for:** Developers parsing `POST /v1/deductions` JSON who need paycheque lines, audit factors, and identity metadata.

**When you finish:** You can map each top-level key to T4127 (Payroll Deductions Formulas) outputs and explain `breakdown` to finance staff.

The payroll **effective date** in `as_of` selects the enacted **rule set** (`rule_set_version`). **Proration** sets `prorated_rules_applied` when Option 1 adjusts mid-year rates. The breakdown includes BPAF (Basic Personal Amount Factor). A **claim code** (TD1 box) on the request maps to `TC` and `TCP`.

Every successful calculation returns the same envelope shape as WASM and the Rust crate. Amounts are decimal strings, never JSON numbers.

## Top-level identity

| Field | Meaning |
| --- | --- |
| `rule_set_version` | Enacted rule bundle id (for example `2026-01-01`). Tied to **effective date** resolution on the request. |
| `engine_version` | Semantic engine release (matches crate `0.1.0`). |
| `engine_build_sha256` | 64-char hex digest of the engine binary used for this response. |
| `prorated_rules_applied` | `true` when Option 1 mid-year **proration** adjusted provincial constants or rates (BC, NL, PE). |

## `employee`

Per-period amounts withheld from the employee. These are what most payroll UIs show on a stub.

| Field | T4127 meaning |
| --- | --- |
| `federal_tax` | Federal income tax for the period (PDOC-style: federal plus additional lines summed for display). |
| `provincial_tax` | Provincial or territorial income tax for the period. |
| `total_tax` | Sum of federal and provincial tax lines as separately rounded cents. |
| `cpp` | Employee CPP (`C`) for the period. |
| `cpp2` | Employee CPP2 (second additional CPP) (`C2`) when pensionable earnings cross YMPE (Year's Maximum Pensionable Earnings). |
| `ei` | Employee EI premium for the period. |
| `qpip` | QPIP (Québec Parental Insurance Plan) employee premium. Zero outside Quebec; QC requests fail before calculation. |
| `total_deductions` | Tax plus CPP plus CPP2 plus EI plus QPIP. |
| `net_pay` | Gross minus `total_deductions` (and any non-tax deductions you modeled in the request). |

## `employer`

Employer matching contributions for the same period.

| Field | Meaning |
| --- | --- |
| `cpp` | Employer CPP match. |
| `cpp2` | Employer CPP2 match. |
| `ei` | Employer EI premium. |
| `qpip` | Employer QPIP. Zero when not applicable. |

## `annual_projection`

Option 1 annualized view after scaling this period to a full year. Useful for sanity checks against T4127 Steps 1 through 5.

| Field | Meaning |
| --- | --- |
| `taxable_income` | Annual taxable income `A`. |
| `federal_tax` | Annual federal tax `T1`. |
| `provincial_tax` | Annual provincial tax `T2`. |
| `cpp` / `cpp2` / `ei` / `qpip` | Annual totals for statutory programs. |

## `breakdown`

Every T4127 factor symbol for this cheque (spec section 9.3). Keys appear even when zero. Values are strings: money with two decimals or rates with four decimal places.

Bookkeepers and auditors use `breakdown` to reconcile a stub to the published formulas without re-running spreadsheets. Examples:

- `A`, `R`, `K`, `T1` trace federal bracket math.
- `V`, `KP`, `T2`, `V2` trace provincial tax and Ontario health premium.
- `BPAF` (Basic Personal Amount Factor) and `TC` / `TCP` tie back to **claim code** tables.
- `P` and `S1` document pay period and Option 2 ratio when applicable.
- `C`, `C2`, `EI`, `F5` tie to CPP, CPP2, and EI chapters.

`employee.total_tax` follows PDOC-style separate rounding. `breakdown.T` follows T4127 Step 6 combined rounding: `round((T1+T2)/P)+L`. They can differ by a cent; both are intentional. Compare a PDOC screen total to `employee.total_tax`; keep `breakdown.T` as the T4127 figure. See CONFORMANCE notes on PDOC mapping.

Full symbol glossary: [glossary.md](./glossary.md).

## `citations`

Array of `{ factor, source_document, source_url }` pointing at the CRA T4127 edition used for key factors (rates, BPAF, claim tables). Use them in compliance exports.

## `warnings`

Non-fatal notices (for example claim code E with Ontario health premium still due). Empty array when nothing applies.

Errors use the same identity fields plus `error.code`; they never include `employee`. See [errors.md](./errors.md).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
