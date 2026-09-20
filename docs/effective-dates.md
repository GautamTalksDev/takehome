# Withholding dates

**Who this is for:** Payroll engineers choosing `as_of` and Option 1 vs Option 2 when mid-year CRA editions change rates.

**When you finish:** You can pick the correct rule set for a pay date, interpret `prorated_rules_applied`, and compare BC Option 1 vs Option 2 withholding.

The payroll **effective date** on a request is the calendar date in `as_of`. It selects which enacted T4127 (Payroll Deductions Formulas) rule bundle applies. **Proration** splits mid-year provincial rates under Option 1 so yearly withholding averages to the true annual rate.

An effective date is not the pay period end date unless you deliberately set it that way.

<figure>
  <img src="/diagrams/02-effective-date-resolution.svg" alt="Effective date resolution" />
  <figcaption>Rule sets are half-open calendar intervals. 2026-06-30 uses the January set. 2026-07-01 uses the July set. The 2027-01-01 set is proposed, not enacted.</figcaption>
</figure>

## Always send `as_of`

The core engine requires `as_of`. The HTTP API fills the UTC request date when you omit it. That date selects rules; the API does not **nearest-match** to another edition.

If you omit `as_of` on a November run while processing a February pay cheque, you get July (or later) tables instead of January tables. Gross and **claim code** stay the same but `R`, `V`, YMPE (Year's Maximum Pensionable Earnings), and BPAF (Basic Personal Amount Factor) can shift. Always set `as_of` to the date your policy says drives withholding (often pay date or period end).

Out-of-range dates return `date_out_of_range` (HTTP 400) with the first supported date. Unknown rule versions on `GET /v1/rules/{version}` return `not_found` (HTTP 404), not a fallback version.

## `prorated_rules_applied`

Some provinces publish true annual rates with Option 1 **proration** that splits the year so withholding averages correctly. Option 2 uses the true mid-year rate in both halves.

<figure>
  <img src="/diagrams/03-proration.svg" alt="British Columbia Option 1 proration" />
  <figcaption>The true annual rate is 5.60 percent. Option 1 withholds 5.06 percent in the first half and a prorated 6.14 percent in the second half so the year averages correctly. Option 2 uses 5.60 percent in both halves. That split is the OptionScoped schema.</figcaption>
</figure>

When Option 1 proration runs, the response sets `prorated_rules_applied` to `true` and `breakdown.V` reflects the prorated provincial rate. Option 2 leaves the flag `false` and uses the enacted July rate directly.

## British Columbia worked example (2026-08-01)

Biweekly `$800.00`, **claim code** 1 on both TD1 forms, 26 pay periods. Same inputs except `calculation_option`.

**Option 1** (`calculation_option`: `option1`):

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY_HERE' \
  -d '{"as_of":"2026-08-01","province":"BC","pay_period":26,"gross_pay":"800.00","federal_claim_code":1,"provincial_claim_code":1,"calculation_option":"option1"}'
```

Expect `prorated_rules_applied`: `true` and `breakdown.V`: `0.0614` (prorated second-half provincial rate).

**Option 2** (`calculation_option`: `option2`):

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY_HERE' \
  -d '{"as_of":"2026-08-01","province":"BC","pay_period":26,"gross_pay":"800.00","federal_claim_code":1,"provincial_claim_code":1,"calculation_option":"option2"}'
```

Expect `prorated_rules_applied`: `false` and `breakdown.V`: `0.0560` (true annual rate from the July edition). Provincial tax lines can match at low income but diverge as income rises.

List enacted intervals with `GET /v1/rules`. Diff two versions with `GET /v1/rules/diff?from=2026-01-01&to=2026-07-01`.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
