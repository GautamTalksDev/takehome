Canadian payroll deduction formulas (CRA T4127) for Python.

Same engine as the Rust crate and the `takehome-ca` npm package. `calculate`
takes request JSON and returns response JSON. Malformed input is
`{"error":{"code":"malformed_json","message":"..."}}` — it does not raise.

```python
import json
import takehome_ca

response = json.loads(
    takehome_ca.calculate(
        json.dumps(
            {
                "as_of": "2026-01-15",
                "province": "ON",
                "pay_period": 52,
                "gross_pay": "1000.00",
            }
        )
    )
)
print(response["employee"]["net_pay"], takehome_ca.engine_build_sha())
```

Publish: create a GitHub Release. `.github/workflows/publish.yml` uses PyPI Trusted Publishing (OIDC). Do not use `PYPI_API_TOKEN` or `scripts/publish-packages.sh` from a laptop.
