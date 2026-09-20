# API reference

**Who this is for:** Developers integrating the hosted Takehome API who need every route, field, and error code in one place.

**When you finish:** You can call any documented route with the correct method, body shape, and typed error handling.

Responses follow T4127 (Payroll Deductions Formulas). Year projection responses may cite YMPE (Year's Maximum Pensionable Earnings) and CPP2 (second additional CPP) cap fields.

Generated from `services/api/openapi.json` (OpenAPI 3.0.3). Regenerate with `node scripts/generate-api-reference.mjs`.

Base URL: https://takehome.gautamkhosla.com

API version: 0.1.0

Authenticated routes expect `Authorization: Bearer np_test_…` or `np_live_…` unless noted.

## GET /health

Liveness only. No versions, dependencies, or environment.

Authentication: none.

### Responses

#### HTTP 200

OK

Body schema: HealthResponse

- `status` ("ok", required)

#### HTTP 503

Engine failed the probe

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 503.

## GET /openapi.json

OpenAPI document generated from the same schemas as this service.

Authentication: none.

### Responses

#### HTTP 200

OpenAPI envelope

Body schema: OpenApiResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `document` (object, required)

## POST /v1/billing/checkout

Stripe Checkout session in CAD for a published self-serve tier.

### Request body

- `plan` ("starter" | "growth" | "business", required)
- `success_url` (string, optional)
- `cancel_url` (string, optional)

### Responses

#### HTTP 200

Checkout URL

Body schema: CheckoutResponse

- `url` (string, required)
- `currency` ("cad", required)
- `plan` (string, required)

#### HTTP 400

Unknown plan

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401, 429.

## POST /v1/billing/webhook

Stripe webhook. Upgrades the plan. Never invents overage.

Authentication: none.

### Responses

#### HTTP 200

Received

Body schema: WebhookResponse

- `received` (boolean, required)

#### HTTP 400

Invalid event

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400.

## GET /v1/changes

Machine-readable change log.

Authentication: none.

### Responses

#### HTTP 200

Changes

Body schema: ChangesResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `feed_title` (string, optional)
- `feed_link` (string, optional)
- `feed_description` (string, optional)
- `items` (array, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 429.

## GET /v1/changes.rss

RSS 2.0 change log.

Authentication: none.

### Responses

#### HTTP 200

RSS

Body: RSS 2.0 XML string.

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 429.

## GET /v1/conformance

PDOC conformance record, including every disagreement.

Authentication: none.

### Responses

#### HTTP 200

Conformance

Body schema: ConformanceResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `record` (object, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 429.

## POST /v1/deductions

Calculate T4127 payroll deductions for one pay period.

Requires a Bearer np_test_ or np_live_ key. Metering counts calculations, not HTTP requests. A batch of 1000 on POST /v1/deductions/batch counts 1000. Test keys are free and unmetered. Live keys hard-stop at the plan limit; Takehome does not surprise-bill.

### Request body

T4127 request. Unknown fields are rejected by name. as_of is required by core; if omitted here the Worker sets the UTC request date and does not nearest-match a rule set.

- `as_of` (string, optional)
- `province` (string, required)
- `pay_period` (integer, required)
- `gross_pay` (string, required) Decimal money as a lexical string. Never a JSON number.
- `calculation_option` (string, optional)
- `pensionable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `insurable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `federal_claim_code` (object, optional)
- `provincial_claim_code` (object, optional)
- `federal_tc` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `provincial_tcp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `cpp_months` (integer, optional)
- `k2_method` (string, optional)
- `rounding_compat` (string, optional)
- `ytd_pensionable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_insurable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_cpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_cpp2` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_ei` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_federal_tax` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_provincial_tax` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `pay_periods_elapsed` (integer, optional)
- `bonus` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `retroactive_pay` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `bonus_method` (string, optional)
- `ytd_bonus` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `f5b_ytd` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `most_recent_i` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `rpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `bonus_rrsp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_bonus_rrsp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_income` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_rpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_union_dues` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `f5a_ytd` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `commission_income` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `commission_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `estimated_annual_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpp2` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpip` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `prior_province` (string, optional)
- `date_of_birth` (string, optional)
- `cpp_exempt` (boolean, optional)
- `cpp_election_after_65` (boolean, optional)
- `additional_tax_requested` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `taxable_benefits` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `union_dues` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `alimony` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `child_care_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `prescribed_zone_deduction` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `lcf_purchase` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `lcp_purchase` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `dependants_under_19` (integer, optional)
- `dependants_disabled` (integer, optional)

### Responses

#### HTTP 200

Deduction

Body schema: DeductionResponse

- `rule_set_version` (string, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `prorated_rules_applied` (boolean, optional)
- `employee` (object, required)
- `employer` (object, required)
- `annual_projection` (object, optional)
- `breakdown` (object, optional)
- `citations` (array, optional)
- `warnings` (array, optional)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 402

Plan calculation limit reached

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 422

Jurisdiction not supported

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401, 402, 422, 429.

## POST /v1/deductions/batch

Calculate T4127 payroll deductions for a pay run of up to 1000 employees.

Requires a Bearer np_test_ or np_live_ key. One HTTP call, up to 1000 calculations, metered as N not 1. Each result is {ok, response} or {error}; one bad employee record does not abort the others. If the live plan cannot cover the whole batch, the call is 402 and usage is unchanged.

### Request body

Up to 1000 T4127 calculations. Partial success: each result is {ok, response} or {error}. Item failures do not abort the pay run.

- `requests` (array, required)

### Responses

#### HTTP 200

Pay run results, in input order

Body schema: BatchResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `results` (array, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 400

Rejected request, including batches over 1000

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 402

Plan calculation limit reached; nothing billed

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401, 402, 429.

## POST /v1/deductions/year

Project every pay period in a year with YTD CPP, CPP2, and EI carried forward.

Requires a Bearer np_test_ or np_live_ key. Metered as P calculations, not 1. Identifies the CPP cap period, EI cap period, YMPE crossing, and CPP2 start (spec §21.5). Sum of per-period federal tax versus annual T1 is allowed to differ by one cent per pay period because of per-period rounding.

### Request body

T4127 request. Unknown fields are rejected by name. as_of is required by core; if omitted here the Worker sets the UTC request date and does not nearest-match a rule set.

- `as_of` (string, optional)
- `province` (string, required)
- `pay_period` (integer, required)
- `gross_pay` (string, required) Decimal money as a lexical string. Never a JSON number.
- `calculation_option` (string, optional)
- `pensionable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `insurable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `federal_claim_code` (object, optional)
- `provincial_claim_code` (object, optional)
- `federal_tc` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `provincial_tcp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `cpp_months` (integer, optional)
- `k2_method` (string, optional)
- `rounding_compat` (string, optional)
- `ytd_pensionable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_insurable_earnings` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_cpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_cpp2` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_ei` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_federal_tax` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_provincial_tax` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `pay_periods_elapsed` (integer, optional)
- `bonus` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `retroactive_pay` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `bonus_method` (string, optional)
- `ytd_bonus` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `f5b_ytd` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `most_recent_i` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `rpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `bonus_rrsp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_bonus_rrsp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_income` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_rpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_union_dues` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `f5a_ytd` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `commission_income` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `commission_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `estimated_annual_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpp` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpp2` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `ytd_qpip` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `prior_province` (string, optional)
- `date_of_birth` (string, optional)
- `cpp_exempt` (boolean, optional)
- `cpp_election_after_65` (boolean, optional)
- `additional_tax_requested` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `taxable_benefits` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `union_dues` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `alimony` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `child_care_expenses` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `prescribed_zone_deduction` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `lcf_purchase` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `lcp_purchase` (string, optional) Decimal money as a lexical string. Never a JSON number.
- `dependants_under_19` (integer, optional)
- `dependants_disabled` (integer, optional)

### Responses

#### HTTP 200

Year projection

Body schema: YearResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `periods` (array, required)
- `totals` (YearTotals, required)
- `annual_t1` (string, required) Decimal money as a lexical string. Never a JSON number.
- `federal_tax_sum_tolerance` (string, required) Decimal money as a lexical string. Never a JSON number.
- `cpp_cap_period` (integer | null, optional)
- `ei_cap_period` (integer | null, optional)
- `cpp2_start_period` (integer | null, optional)
- `ympe_cross_period` (integer | null, optional)
- `ympe` (string, optional) Decimal money as a lexical string. Never a JSON number.

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 402

Plan calculation limit reached; nothing billed

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 422

Jurisdiction not supported

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

Header `X-Usage-Limit`: Monthly live calculation cap, or unlimited for np_test_ keys.
Header `X-Usage-Remaining`: Calculations remaining this UTC month, or unlimited.
Header `X-Usage-Reset`: Next UTC month start, when the live meter resets.

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401, 402, 422, 429.

## GET /v1/jurisdictions

Provinces and territories. Quebec is listed as unsupported.

Authentication: none.

### Responses

#### HTTP 200

Jurisdictions

Body schema: JurisdictionsResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `jurisdictions` (array, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 429.

## GET /v1/keys

Active key prefixes for this account. Secrets are never returned.

### Responses

#### HTTP 200

Prefixes only

Body schema: KeyListResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `keys` (array, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401.

## POST /v1/keys/revoke

Revoke keys of the given kind immediately. A lost key is regenerated, never retrieved.

### Request body

- `kind` ("test" | "live", required)

### Responses

#### HTTP 200

Revoked

Body schema: KeyRevokeResponse

- `kind` ("test" | "live", required)
- `revoked` (true, required)

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401.

## POST /v1/keys/rotate

Issue a new key of the given kind. The previous key is revoked immediately. The new secret is shown once.

### Request body

- `kind` ("test" | "live", required)

### Responses

#### HTTP 200

New secret, shown once

Body schema: KeyRotateResponse

- `kind` ("test" | "live", required)
- `key` (string, required)
- `message` (string, required)

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401.

## GET /v1/rules

Embedded T4127 rule-set editions.

Authentication: none.

### Responses

#### HTTP 200

Rule set list

Body schema: RulesListResponse

- `rule_set_version` (string, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `rule_set_versions` (array, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 429.

## GET /v1/rules/diff

Field-by-field comparison of two embedded T4127 editions.

Authentication: none.

### Parameters

- `from` (query, required): string
- `to` (query, required): string

### Responses

#### HTTP 200

Rule-set diff

Body schema: RulesDiffResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `from` (string, required)
- `to` (string, required)
- `changed` (array, required)
- `fields` (object, required)
- `cpp` (array, required)
- `ei` (array, required)
- `qpip` (array, required)

#### HTTP 400

Missing versions

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 404

Unknown version

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 404, 429.

## GET /v1/rules/{version}

One embedded rule-set edition. Unknown versions 404; never nearest-match.

Authentication: none.

### Parameters

- `version` (path, required): string

### Responses

#### HTTP 200

Rule set

Body schema: RuleVersionResponse

- `rule_set_version` (string, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `effective_from` (string, required)
- `effective_to` (string | null, optional)
- `status` ("enacted" | "proposed", required)

#### HTTP 404

Unknown version

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 404, 429.

## POST /v1/signup

Start signup. Email, verify, keys. No sales call.

Authentication: none.

### Request body

- `email` (string, required)

### Responses

#### HTTP 200

Verification email sent

Body schema: SignupResponse

- `message` (string, required)

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 429.

## GET /v1/signup/verify

Verify email and issue np_test_ plus np_live_ keys.

Authentication: none.

### Parameters

- `token` (query, required): string

### Responses

#### HTTP 200

Keys issued once

Body schema: VerifyResponse

- `test_key` (string, required)
- `live_key` (string, required)
- `message` (string, required)

#### HTTP 400

Invalid token

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 429

Rate limited

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 429.

## GET /v1/webhooks

Webhook endpoints for this account.

### Responses

#### HTTP 200

Endpoints

Body schema: WebhookListResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `webhooks` (array, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401.

## POST /v1/webhooks

Register an HTTPS endpoint. Signing secret is shown once.

The whsec_ secret is returned only on create. Deliveries are HMAC-SHA256 over t + "." + body as takehome-signature: t=<unix>,v1=<hex>. Receivers must reject |now - t| > 300 seconds (5 minutes).

### Request body

- `url` (string, required)

### Responses

#### HTTP 200

Created

Body schema: WebhookCreateResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `id` (string, required)
- `url` (string, required)
- `secret` (string, required)
- `created_at` (string, required)

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401.

## GET /v1/webhooks/deliveries

Delivery log for this account.

### Responses

#### HTTP 200

Log

Body schema: WebhookDeliveriesResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `deliveries` (array, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401.

## POST /v1/webhooks/deliveries/{id}/replay

Re-send one signed payload.

### Parameters

- `id` (path, required): string

### Responses

#### HTTP 200

Replayed

Body schema: WebhookReplayResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `id` (string, required)
- `endpoint_id` (string, optional)
- `event` (string, optional)
- `status` (string, required)
- `attempts` (integer, required)
- `last_error` (string | null, optional)
- `replay_of` (string | null, optional)
- `created_at` (string, optional)
- `delivered_at` (string | null, optional)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 404

Unknown delivery

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401, 404.

## POST /v1/webhooks/dispatch

Fan out a rule_set.changed event to every endpoint on this account.

### Request body

- `from` (string, required)
- `to` (string, required)

### Responses

#### HTTP 200

Deliveries

Body schema: WebhookDispatchResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `from` (string, required)
- `to` (string, required)
- `changed` (array, required)
- `deliveries` (array, required)

#### HTTP 400

Rejected request

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 404

Unknown rule set

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 400, 401, 404.

## DELETE /v1/webhooks/{id}

Delete a webhook endpoint on this account. Unknown or foreign IDs are 404.

### Parameters

- `id` (path, required): string

### Responses

#### HTTP 200

Deleted

Body schema: WebhookDeleteResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `deleted` (boolean, required)
- `id` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 404

Unknown webhook

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401, 404.

## GET /v1/webhooks/{id}

One webhook endpoint on this account. Unknown or foreign IDs are 404.

### Parameters

- `id` (path, required): string

### Responses

#### HTTP 200

Endpoint

Body schema: WebhookGetResponse

- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)
- `id` (string, required)
- `url` (string, required)
- `created_at` (string, required)

#### HTTP 401

Missing or unknown key

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

#### HTTP 404

Unknown webhook

Body schema: ErrorEnvelope

- `error` (ErrorBody, required)
- `rule_set_version` (string | null, required)
- `engine_version` (string, required)
- `engine_build_sha256` (string, required)

### Error responses on this route

HTTP statuses that return `ErrorEnvelope`: 401, 404.

## Error codes

Every error JSON matches `ErrorEnvelope`: `error` (`code`, `message`, `docs`) plus `rule_set_version`, `engine_version`, and `engine_build_sha256`.

### `batch_too_large`

HTTP 400.

**Cause:** POST /v1/deductions/batch carried more than 1000 requests.

**What to do:** Split the pay run into chunks of at most 1000 calculations.

### `date_out_of_range`

HTTP 400.

**Cause:** as_of falls before the first enacted rule set or after the last covered date.

**What to do:** Pick a supported calendar date. The API does not snap to the nearest rule set.

### `engine`

HTTP 500.

**Cause:** An internal engine or health probe failure occurred.

**What to do:** Retry once. If it persists, contact support with rule_set_version and engine_build_sha256 from the envelope.

### `invalid_request`

HTTP 400.

**Cause:** The JSON parsed but violates request rules (shape, enums, or business constraints).

**What to do:** Read error.message. Fix the named constraint and retry.

### `jurisdiction_not_supported`

HTTP 422.

**Cause:** The province code is not calculated (for example QC).

**What to do:** Use GET /v1/jurisdictions. Do not treat a partial total as Quebec payroll tax.

### `malformed_json`

HTTP 400.

**Cause:** The body is not valid JSON or exceeds depth or size limits.

**What to do:** Fix syntax. Send application/json. Keep payloads within documented limits.

### `method_not_allowed`

HTTP 405.

**Cause:** The HTTP method does not match the route contract.

**What to do:** Use the method shown in this reference for the path.

### `not_found`

HTTP 404.

**Cause:** The path, rule version, webhook id, or delivery id does not exist for this account.

**What to do:** Fix the URL or id. Cross-account webhook access returns 404, not 403.

### `payload_too_large`

HTTP 413.

**Cause:** The request body exceeded the Worker byte limit.

**What to do:** Shrink the batch or payload. Prefer batch for many employees.

### `plan_limit`

HTTP 402.

**Cause:** The live key reached its UTC-month calculation cap.

**What to do:** Upgrade via POST /v1/billing/checkout or wait until X-Usage-Reset. Failed calls are not billed.

### `rate_limited`

HTTP 429.

**Cause:** Too many HTTP requests in the rate-limit window for this route or IP.

**What to do:** Backoff and retry. Metering limits are separate; see rate-limits-and-billing.md.

### `rule`

HTTP 400.

**Cause:** The engine rejected rule lookup or diff input.

**What to do:** Confirm rule set versions with GET /v1/rules. Use enacted dates only unless testing proposed sets.

### `store`

HTTP 503.

**Cause:** Account, usage, or key storage failed (fail closed).

**What to do:** Retry later. Live metering outages do not serve unmetered calculations.

### `unauthorized`

HTTP 401.

**Cause:** Missing Bearer key, wrong prefix, or revoked secret.

**What to do:** Send Authorization: Bearer np_test_… or np_live_…. Rotate at POST /v1/keys/rotate if lost.

### `unknown_field`

HTTP 400.

**Cause:** The JSON body includes a property name the engine does not recognize.

**What to do:** Remove or rename the field. See request-fields.md for the allowed set.

### `unknown_plan`

HTTP 400.

**Cause:** Checkout plan is not starter, growth, or business.

**What to do:** Send a published plan name from docs/pricing.md.

### `unsupported_media_type`

HTTP 415.

**Cause:** Content-Type was not application/json where required.

**What to do:** Set content-type: application/json on POST bodies.

**Last reviewed:** 2026-09-20
**Engine:** 0.1.0
