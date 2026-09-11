# Netpay

Canadian payroll deduction calculation. Effective-date versioned. Apache-2.0.

```bash
curl -X POST https://api.netpay.ca/v1/deductions \
  -H 'content-type: application/json' \
  -d '{"province":"ON","pay_period":26,"gross_pay":"2500.00"}'
```

Every response names the rule set version that answered it. Ask for a date in
February 2026 in November 2026 and you get February's rules.

- Conformance record (including every disagreement with CRA PDOC): /conformance
- Machine-readable change log: /changes
- Pre-registered stop condition: [KILL-TEST.md](./KILL-TEST.md)

Netpay is a calculation service, not a payroll provider. It does not file
returns or move money. The CRA's Payroll Deductions Online Calculator is the
authoritative source.
