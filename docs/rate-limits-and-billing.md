# Rate limits and billing

**Who this is for:** Teams running live pay batches who need to predict monthly cost and HTTP headers on every calculation.

**When you finish:** You know what increments usage, how batch size counts, and what happens at the plan cap.

Takehome meters **calculations**, not HTTP requests. Pricing is CAD and published in [pricing.md](./pricing.md).

## What counts as one calculation

| Call | Count |
| --- | --- |
| `POST /v1/deductions` | 1 per successful `200` response |
| `POST /v1/deductions/batch` | N = length of `requests` array on success |
| `POST /v1/deductions/year` | One per successful year projection |

Examples:

- A batch of 500 employees in one HTTP call counts **500** calculations.
- A batch of 1000 counts 1000 (the maximum array size).
- Failed validations, `401`, `402`, `422`, and `4xx` engine errors do not increment usage after a rejection.
- If a batch would exceed remaining live quota, the whole call returns `plan_limit` (402) and **nothing** is billed.

Quebec (`QC`) `jurisdiction_not_supported` responses are not successful calculations and are not billed.

## Usage headers

Successful deduction and batch responses include:

| Header | Meaning |
| --- | --- |
| `X-Usage-Limit` | Monthly live calculation cap for the plan, or `unlimited` for `np_test_` keys |
| `X-Usage-Remaining` | Calculations left in the current UTC month |
| `X-Usage-Reset` | ISO timestamp when the UTC month meter resets |

Test keys return unlimited headers. Live keys return integers derived from the plan row in [pricing.md](./pricing.md).

## Hard stop at the limit

Live keys (`np_live_`) stop with HTTP 402 and `error.code` `plan_limit` when the UTC-month tally would exceed the plan. There is no overage billing and no silent throttle to cheaper math.

Upgrade path: `POST /v1/billing/checkout` with Bearer live key and `{ "plan": "starter" | "growth" | "business" }`. Stripe Checkout charges CAD.

Developer tier: $0 with 1,000 live calculations per UTC month.

## Test keys

`np_test_` keys run the **same engine** and return the same JSON as live keys for identical inputs. They are free and unmetered. Use them for CI, staging, and the five-minute onboarding test.

HTTP rate limits (`rate_limited`, 429) still apply separately from calculation metering. Back off on 429 regardless of key kind.

## Metering outages

If usage storage fails, live calculation routes return `store` (503) and do not perform unmetered live math. See [errors.md](./errors.md) and ADR-005.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
