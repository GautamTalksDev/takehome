# How the calculation works

**Who this is for:** Developers and technical payroll staff who want T4127 (Payroll Deductions Formulas) Option 1 in plain language before they read factor symbols.

**When you finish:** You can walk through the six Chapter 4 steps and reconcile one Ontario weekly paycheque to `net_pay` `800.79`.

Chapter 4 describes **Option 1**: you project annual income from this pay period, compute annual tax, then divide back to the cheque. Chapters 6 and 7 add CPP and EI for the period.

## Six Option 1 steps (Chapter 4)

1. **Annual taxable income (`A`).** Scale this period’s pensionable and taxable earnings to a full year. Subtract CPP additional contribution deductions (`F5`, `F5A`, `F5B`) per Step 1. **`P`** is the number of pay periods in the year (52 for weekly).

2. **Federal annual tax (`T1`).** Pick the federal rate **`R`** and constant **`K`** for **`A`**. Compute basic tax **`T3`**, then subtract credits **`K1`**, **`K2`**, **`K4`** (and **`K3`**, **`LCF`** when they apply). The result is annual federal tax **`T1`**.

3. **Federal per-period tax.** Divide **`T1`** by **`P`** and round to cents. Add any extra tax **`L`** the employee requested on TD1.

4. **Provincial annual basic tax (`T4`).** Use provincial rate **`V`** and constant **`KP`**. Apply the same credit pattern at the provincial level (`K1P`, `K2P`, and related factors).

5. **Provincial annual tax (`T2`).** Start from **`T4`**. Add Ontario surtax **`V1`** and health premium **`V2`** when they apply. Apply reductions **`S`** and **`Y`** where relevant. Other provinces skip lines that do not apply.

6. **Tax for the pay period (`T`).** Combine federal and provincial annual tax: round **`(T1 + T2) / P`**, then add **`L`**. Employee **`total_tax`** on the API is often the sum of separately rounded federal and provincial lines (PDOC-style); **`breakdown.T`** follows the T4127 Step 6 quotient.

After Chapter 4, the engine computes **`C`** (CPP), **`C2`**, and **`EI`**. Factor **`C2`** is CPP2 (second additional CPP) when pensionable earnings exceed YMPE (Year's Maximum Pensionable Earnings).

**net pay = gross pay − total tax − CPP − C2 − EI**

## Worked example: Ontario, weekly, $1000, claim code 1 on the TD1

Inputs match the first-call vector:

| Field | Value |
| --- | --- |
| `as_of` | `2026-01-15` (selects rule set `2026-01-01`) |
| `province` | `ON` |
| `pay_period` | `52` |
| `gross_pay` | `1000.00` |
| `federal_claim_code` | `1` |
| `provincial_claim_code` | `1` |

A **claim code** (TD1 box) maps personal amounts to **`TC`** and **`TCP`**. Code 1 is the default basic personal amount for 2026.

### Step 1: `A`

The engine annualizes this cheque and lands on **`A` = `51514.84`**.

### Steps 2 and 3: Federal

With **`R` = `0.1400`**, credits yield annual **`T1` = `4243.86`**. Per period federal income tax rounds to **`81.61`**.

### Steps 4 and 5: Provincial

Ontario adds health premium **`V2`** in the annual provincial stack. Annual **`T2` = `2381.51`**, which rounds to **`45.80`** per week.

### Step 6: Tax total

Employee **`total_tax` = `127.41`** (`81.61` + `45.80`). **`breakdown.T`** matches the Step 6 combined quotient (`127.41` here).

### CPP and EI

For this cheque: **`C` (CPP) = `55.50`**, **`C2` = `0.00`**, **`EI` = `16.30`**.

### Net pay

**`total_deductions` = `199.21`**  
**`net_pay` = `800.79`**

That is **`1000.00 − 199.21`**. The same request through the API or the [Ontario calculator](https://takehome.gautamkhosla.com/calculators/on/) returns these cents as strings.

For symbol definitions, see [glossary.md](./glossary.md).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
