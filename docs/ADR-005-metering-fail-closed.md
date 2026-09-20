# ADR-005: Metering fails closed when D1 is unavailable

**Who this is for:** API maintainers handling live-key usage limits and outages.

**When you finish:** You know why a D1 failure returns 503 without calculation results.

## Status

Accepted (A10:2025)

## Context

Live keys are metered in D1 (`usage.calculations` per account and month).
If the Worker cannot read or write that row, two policies exist:

1. **Fail open:** run the calculation anyway, skip `addUsage`. An
   attacker who can take D1 down (or who waits for an outage)
   gets unmetered access for the duration. The plan limit is then a
   best-effort counter, not a control.
2. **Fail closed:** refuse the request with a typed `503` / `store`
   error and do not return `employee`. Downtime is visible. Unmetered
   access is not.

Test keys are not billed, but they still go through the same store.
A usage-query failure is treated the same way: if we cannot talk to
the meter, we do not serve a calculation.

## Decision

Fail closed. `getUsage` or `addUsage` throwing is a `503` with
`error.code = store`. The client message does not include the D1
error. The calculation result is not in the response. When `getUsage`
fails, `engine.calculate` is not called. When `addUsage` fails after a
successful calculate, the result is discarded. Serving it would be
failing open for that request.

Do not catch a usage error and continue. Do not substitute `used = 0`
on a store failure.

## Consequences

- A D1 outage is a Worker outage for `/v1/deductions*`. That is the
  price of the meter being a security control.
- Operators see `503` / `store` and the server-side exception log,
  not silent free calculations.
- Revenue risk from an outage is deferred demand, not unpaid usage.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
