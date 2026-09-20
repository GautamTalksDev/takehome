# Pricing

**Who this is for:** Buyers choosing a live plan for T4127 (Payroll Deductions Formulas) calculations.

**When you finish:** You know early access is free, the monthly live quota, and what paid tiers will cost later.

Takehome is a T4127 calculation service. Supported jurisdictions are the Table 8.1
provinces and territories plus Outside Canada. See [jurisdictions](jurisdictions.md)
and `GET /v1/jurisdictions`.

## Early access (now)

Everything is free. No card required. See [ADR-006](ADR-006-billing-deferred.md).

| Plan | Price | Live calculations / month |
| --- | --- | --- |
| Developer (every live account) | $0 CAD | 100,000 |
| Test keys (`np_test_`) | $0 CAD | unmetered |

Live keys (`np_live_`) count **calculations**, not HTTP requests. A batch of
500 on `POST /v1/deductions/batch` counts 500. At the plan limit the API returns
`plan_limit` (402). A batch that would exceed remaining quota is rejected whole.
There is no overage and no surprise bill. There is no paid upgrade path until
billing returns.

`POST /v1/billing/checkout` returns `501` / `billing_unavailable`.
`POST /v1/billing/webhook` returns `404`.

## Paid tiers (later)

Published CAD prices for when checkout turns on. Not for sale yet.

| Plan | Price | Live calculations / month |
| --- | --- | --- |
| Starter | $49 CAD | 25,000 |
| Growth | $149 CAD | 150,000 |
| Business | $399 CAD | 1,000,000 |

There is no sales call.

## Quebec is not a priced calculation

Quebec is not calculated. A `QC` request fails with
`JurisdictionNotSupported`: it is not billed as a successful deduction, and
it never returns federal-only tax with T2 = 0. Quebec / QPP / QPIP (Québec Parental Insurance Plan) is on the
roadmap; until it ships, do not treat a Takehome total as Quebec payroll tax.

Signup: email, verify, keys, first call. See https://takehome.gautamkhosla.com/signup/

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
