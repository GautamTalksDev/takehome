# Quickstart

**Who this is for:** Developers who want a test key and a verified T4127 (Payroll Deductions Formulas) calculation in one sitting.

**When you finish:** You hold an `np_test_` key and you have seen a full deduction response with `net_pay` `800.79`.

Takehome implements CRA payroll withholding for Table 8.1 provinces and territories plus Outside Canada. Quebec (`QC`) is not supported yet.

## 1. Sign up

Open [signup](https://takehome.gautamkhosla.com/signup/) or call the API:

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/signup \
  -H 'content-type: application/json' \
  -d '{"email":"you@example.com"}'
```

You receive a short JSON message that a verification email was sent.

## 2. Verify email

Open the link in the email. It hits `GET /v1/signup/verify?token=…` and returns both keys once:

```json
{
  "test_key": "np_test_0123456789abcdef0123456789abcdef",
  "live_key": "np_live_0123456789abcdef0123456789abcdef",
  "message": "Store these keys securely. They are shown once."
}
```

Copy the `np_test_` key for integration work. Live keys meter calculations; test keys do not.

## 3. First calculation

Ontario weekly `$1000.00` on `2026-01-15` with claim code 1 on both TD1 forms is the canonical smoke test:

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY_HERE' \
  -d '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}'
```

A **claim code** (TD1 box) maps personal amounts to federal and provincial credits. Code 1 is the default basic personal amount for 2026.

## 4. Full response (abbreviated citations)

A successful `200` body looks like this (citations array shortened):

```json
{
  "rule_set_version": "2026-01-01",
  "engine_version": "0.1.0",
  "engine_build_sha256": "b8a89f98fa2fcf25715aa6171bdf0b2d2b00ad2fe595b1187c119bb7cd9c77bd",
  "prorated_rules_applied": false,
  "employee": {
    "federal_tax": "81.61",
    "provincial_tax": "45.80",
    "total_tax": "127.41",
    "cpp": "55.50",
    "cpp2": "0.00",
    "ei": "16.30",
    "qpip": "0.00",
    "total_deductions": "199.21",
    "net_pay": "800.79"
  },
  "employer": {
    "cpp": "55.50",
    "cpp2": "0.00",
    "ei": "22.82",
    "qpip": "0.00"
  },
  "annual_projection": {
    "taxable_income": "51514.84",
    "federal_tax": "4243.86",
    "provincial_tax": "2381.51",
    "cpp": "2886.00",
    "cpp2": "0.00",
    "ei": "847.60",
    "qpip": "0.00"
  },
  "breakdown": {
    "A": "51514.84",
    "R": "0.1400",
    "K": "0.00",
    "K1": "2303.28",
    "K2": "454.80",
    "K4": "210.14",
    "T3": "4243.86",
    "T1": "4243.86",
    "V": "0.0505",
    "KP": "0.00",
    "K1P": "655.94",
    "K2P": "164.05",
    "T4": "1781.51",
    "V1": "0.00",
    "V2": "600.00",
    "S": "0.00",
    "T2": "2381.51",
    "T": "127.41",
    "P": "52",
    "BPAF": "16452.00",
    "QPIP": "0.00"
  },
  "citations": [
    {
      "factor": "R",
      "source_document": "T4127 Payroll Deductions Formulas - 122nd Edition Effective January 1, 2026",
      "source_url": "https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html"
    }
  ],
  "warnings": []
}
```

Confirm `"net_pay": "800.79"`. Any other cents means the wrong inputs or an unexpected engine build.

## Identity fields on every response

**`rule_set_version`** names the enacted CRA rule bundle selected by `as_of` (here `2026-01-01` for January through June 2026). Store it on pay stubs and audit logs so you can replay the same withholding later.

**`engine_build_sha256`** is the SHA-256 digest of the Rust core binary that performed the math. It matches WASM and native builds when they share a release. Log it beside `rule_set_version` to prove which code path produced the cheque.

See [response-fields.md](./response-fields.md) for employee amounts and [effective-dates.md](./effective-dates.md) for how `as_of` picks the rule set.

## Local API

When you run `npm run local` under `services/api`, point curl at `http://127.0.0.1:8787` and use the same paths.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
