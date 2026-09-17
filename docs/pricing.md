# Pricing

Takehome is a T4127 calculation service. Supported jurisdictions are the Table 8.1
provinces and territories plus Outside Canada. See [jurisdictions](jurisdictions.md)
and `GET /v1/jurisdictions`.

Prices are CAD, published, and self-serve. There is no sales call.

## Quebec is not a priced calculation

Quebec is not calculated. A `QC` request fails with
`JurisdictionNotSupported` — it is not billed as a successful deduction, and
it never returns federal-only tax with T2 = 0. Quebec / QPP / QPIP is on the
roadmap; until it ships, do not treat a Takehome total as Quebec payroll tax.

## Live plans (UTC month)

| Plan | Price | Live calculations / month |
| --- | --- | --- |
| Developer | $0 CAD | 1,000 |
| Starter | $49 CAD | 25,000 |
| Growth | $149 CAD | 150,000 |
| Business | $399 CAD | 1,000,000 |

Test keys (`np_test_`) run the same engine, are $0, and are not metered.

Live keys (`np_live_`) count **calculations**, not HTTP requests. A future batch
of 500 counts 500. At the plan limit the API returns `plan_limit` (402) with an
upgrade link. There is no overage and no surprise bill.

Upgrade: `POST /v1/billing/checkout` with a Bearer live key and
`{ "plan": "starter" | "growth" | "business" }`. Stripe Checkout is CAD.

Signup: email, verify, keys, first call. See https://takehome.gautamkhosla.com/signup/
