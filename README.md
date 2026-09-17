# Takehome

Shows its work.

Canadian payroll deduction calculation. Effective-date versioned. Apache-2.0.

```bash
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY' \
  -d '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}'
```

`employee.net_pay` is `800.79`. Get a key at https://takehome.gautamkhosla.com/signup/ — docs
at https://takehome.gautamkhosla.com/docs/. The five-minute test is
[docs/FIVE-MINUTE-TEST.md](docs/FIVE-MINUTE-TEST.md).

Every response names the rule set version that answered it. Ask for a date in
February 2026 in November 2026 and you get February's rules.

```bash
curl https://takehome.gautamkhosla.com/v1/jurisdictions
```

Quebec is listed there as unsupported. A `QC` deductions request returns
`JurisdictionNotSupported`; it does not return federal-only tax. Details:
[docs/jurisdictions.md](docs/jurisdictions.md), [docs/pricing.md](docs/pricing.md).

- Conformance record (including every disagreement with CRA PDOC): /conformance
- Machine-readable change log: /changes
- Pre-registered stop condition: [KILL-TEST.md](./KILL-TEST.md)
- npm: [`takehome-ca`](https://www.npmjs.com/package/takehome-ca) (WASM)
- PyPI: [`takehome-ca`](https://pypi.org/project/takehome-ca/)

Takehome is a calculation service, not a payroll provider. It does not file
returns or move money. The CRA's Payroll Deductions Online Calculator is the
authoritative source.
