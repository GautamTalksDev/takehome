# Pricing

Netpay is a T4127 calculation service. Supported jurisdictions are the Table 8.1
provinces and territories plus Outside Canada. See [jurisdictions](jurisdictions.md)
and `GET /v1/jurisdictions`.

## Quebec is not a priced calculation

Quebec is not calculated. A `QC` request fails with
`JurisdictionNotSupported` — it is not billed as a successful deduction, and
it never returns federal-only tax with T2 = 0. Quebec / QPP / QPIP is on the
roadmap; until it ships, do not treat a Netpay total as Quebec payroll tax.
