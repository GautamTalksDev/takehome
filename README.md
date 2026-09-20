# Takehome

**Who this is for:** Developers and payroll integrators who need CRA T4127 (Payroll Deductions Formulas) math with cited factors, not a black box.

**When you finish:** You know what Takehome is, you have run the canonical call, and you have read the limits before you ship.

Takehome is a Canadian payroll deduction engine that implements T4127 Option 1 for thirteen provinces and territories and returns every factor it used.

## Product

- **Effective date versioning:** Each request carries an `as_of` calendar date; the response names the rule set version that answered.
- **Published conformance:** We publish an overall PDOC agreement rate and list every disagreement, not a headline percentage alone.
- **Permissive licence:** The Rust engine, embedded rule data, and npm/PyPI packages ship under Apache-2.0.
- **Offline browser:** Province calculators run the same WASM engine locally after the first load; gross pay does not leave the browser.

- [API docs](https://takehome.gautamkhosla.com/docs/)
- [npm `takehome-ca`](https://www.npmjs.com/package/takehome-ca) (WASM)
- [PyPI `takehome-ca`](https://pypi.org/project/takehome-ca/)

## Proof

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/deductions \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_test_YOUR_KEY' \
  -d '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}'
```

Replace `np_test_YOUR_KEY` with a test key from [signup](https://takehome.gautamkhosla.com/signup/). The JSON body includes `"net_pay": "800.79"` for Ontario weekly `$1000.00` on `2026-01-15` with federal and provincial claim code 1 (TD1 default personal amount).

Overall agreement with CRA PDOC Option 1 corpora: **[8838/9002](https://takehome.gautamkhosla.com/conformance)**. Machine-readable detail lives at `/conformance` and in [CONFORMANCE.md](./CONFORMANCE.md).

## Caveats

- **Quebec:** `QC` is unsupported. The API returns `JurisdictionNotSupported`; it never returns federal-only tax with provincial tax zeroed.
- **PDOC deltas:** **164** one-cent disagreements with live PDOC remain listed on the conformance page (M-003). We do not hide them in footnotes.
- **2027 preview:** The `2027-01-01` rule set is **proposed**, not enacted. Responses carry a `RULE_SET_PROPOSED` warning until CRA publishes final tables.

**Licence split:** Source code (including `services/api`), rule JSON vendored in the engine, and client packages are [Apache-2.0](./LICENSE-APACHE). The hosted API at `takehome.gautamkhosla.com` is a metered product: the running service is proprietary as a commercial offering, not because its Worker source is withheld. If the hosted service stops, the engine, data, and Worker source stay published under that licence ([KILL-TEST.md](./KILL-TEST.md)). See [docs/pricing.md](docs/pricing.md).

Takehome calculates deductions. It does not file returns, remit source deductions, or move money. CRA PDOC remains the authoritative calculator for employers.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
