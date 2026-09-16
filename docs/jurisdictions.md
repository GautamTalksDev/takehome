# Jurisdictions

Netpay calculates T4127 payroll deductions for the Table 8.1 jurisdictions
plus Outside Canada.

| Code | Name | Status |
|------|------|--------|
| AB | Alberta | supported |
| BC | British Columbia | supported |
| MB | Manitoba | supported |
| NB | New Brunswick | supported |
| NL | Newfoundland and Labrador | supported |
| NS | Nova Scotia | supported |
| NT | Northwest Territories | supported |
| NU | Nunavut | supported |
| ON | Ontario | supported |
| PE | Prince Edward Island | supported |
| QC | Quebec | **not supported** |
| SK | Saskatchewan | supported |
| YT | Yukon | supported |
| OutsideCanada | Outside Canada | supported (federal surtax; T2 = 0) |

## Quebec

Quebec is not T4127 Chapter 4 provincial tax. It uses Revenu Québec formulas
(QPP / QPIP). A request with `"province":"QC"` returns
`EngineError::JurisdictionNotSupported`. It does **not** return a federal-only
answer with T2 = 0 — that would look like a successful calculation and be
wrong.

Roadmap: QPP / QPIP end-to-end is a later milestone. Until then, Quebec's
absence is on this page, on `/v1/jurisdictions`, and on the pricing page.

Machine-readable listing:

```
GET /v1/jurisdictions
```

The payload is [`list_jurisdictions()`](../crates/netpay-core/src/jurisdictions.rs)
(`netpay jurisdictions` prints the same JSON).
