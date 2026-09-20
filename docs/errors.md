# Errors

**Who this is for:** Integrators who branch on HTTP status and `error.code` instead of parsing free-text messages.

**When you finish:** You know why Takehome rejects bad input, why Quebec never returns a fake zero-tax stub, and why unknown JSON keys fail fast.

A **claim code** is the TD1 box value that maps personal amounts to federal and provincial credits. Errors never return partial employee amounts.

Every error returns `ErrorEnvelope`:

```json
{
  "error": {
    "code": "date_out_of_range",
    "message": "as_of 2025-12-31 is before coverage begins on 2026-01-01.",
    "docs": "https://takehome.gautamkhosla.com/what-is-the-t4127"
  },
  "rule_set_version": null,
  "engine_version": "0.1.0",
  "engine_build_sha256": "b8a89f98fa2fcf25715aa6171bdf0b2d2b00ad2fe595b1187c119bb7cd9c77bd"
}
```

`error` always has exactly `code`, `message`, and `docs` (HTTPS link). There are no stack traces, SQL fragments, or internal paths in the body.

## Design choices

### Typed codes, stable HTTP mapping

Each failure maps to one `error.code` and a documented HTTP status. Handlers use `classifyEngineError` so engine messages about unknown fields become `unknown_field` and date coverage gaps become `date_out_of_range`. Clients should switch on `code`, not substring match `message`.

### Quebec is unsupported, not zero tax

`province: "QC"` returns `jurisdiction_not_supported` (HTTP 422). Takehome does not return federal-only tax with `provincial_tax` forced to zero. QPIP (Québec Parental Insurance Plan) and QPP are not approximated. Billing does not count a failed QC call as a successful live calculation.

### Dates are rejected, not snapped

When `as_of` is before the first enacted rule set or after coverage ends, the API returns `date_out_of_range` (HTTP 400). It does not pick the closest January or July edition. The same strict behavior applies to `GET /v1/rules/{version}` for unknown versions (`not_found`, HTTP 404).

### Unknown fields are rejected, not ignored

Extra JSON keys fail deserialization with `unknown_field` (HTTP 400). The engine names the typo in `message`. Silent ignore would let integrators ship wrong withholding for weeks before anyone notices.

### Fail closed on metering storage

If usage storage fails, live keys receive `store` (HTTP 503) and no calculation is served. See ADR-005. Test keys stay unmetered but still need a working engine.

### Batch partial success

`POST /v1/deductions/batch` returns HTTP 200 when the envelope is valid even if some items fail. Each slot is `{ "ok": true, "response": … }` or `{ "error": { … } }`. Whole-batch rejection happens for `batch_too_large`, `plan_limit`, or malformed batch shape.

## Common codes

| Code | HTTP | Typical cause |
| --- | --- | --- |
| `unknown_field` | 400 | Typo in JSON property name |
| `invalid_request` | 400 | Illegal pay period, ambiguous claim code plus TC, bad webhook URL |
| `date_out_of_range` | 400 | `as_of` outside enacted coverage |
| `jurisdiction_not_supported` | 422 | Quebec or other unsupported code |
| `unauthorized` | 401 | Missing or revoked API key |
| `plan_limit` | 402 | Live monthly calculation cap reached |
| `rate_limited` | 429 | Too many HTTP requests |
| `engine` | 500 | Internal engine failure (generic message) |
| `store` | 503 | Storage outage (metering or keys) |

Full list with remediation steps: [api-reference.md](./api-reference.md#error-codes).

Batch item errors reuse the same `error` object shape but omit top-level identity inside the slot (only `error` on failure items).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
