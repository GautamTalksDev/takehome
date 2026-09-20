# ADR-006: Paid billing deferred for free early access

**Who this is for:** Operators deciding when Stripe checkout returns, and
anyone reading the kill-test stop condition after the free launch.

**When you finish:** You know why `/v1/billing/*` is dark, what turns it
back on, and that kill-test route C is withdrawn until then.

## Status

Accepted (2026-09-20)

## Context

The product published CAD self-serve tiers (Starter 49, Growth 149,
Business 399) and Stripe-shaped routes. A production launch with expired
test keys and placeholder `price_*` IDs would leave a live checkout that
fails in an undefined way. Half-wired billing is worse than no billing.

The pre-registered kill test (`KILL-TEST.md`) had three routes. Route C
required three paying customers. With everything free, C is unreachable.
Quietly leaving C in place while removing the only path to pay would
change a pre-registered condition without a record.

## Decision

1. Defer paid billing. Every account stays on the free Developer tier.
2. Free early-access live quota is **100,000** calculations per UTC month
   (was 1,000). Test keys stay unmetered. The hard stop remains.
3. `POST /v1/billing/checkout` returns typed `501` / `billing_unavailable`.
   `POST /v1/billing/webhook` returns `404`. No Stripe secrets in
   production.
4. `services/api/src/stripe.js` stays in the tree, unreferenced by live
   routes, with tests that keep the signature helpers warm.
5. Amend `KILL-TEST.md`: route C withdrawn; A and B unchanged. Dated.
6. Pricing page stays. It says early access is free, states the quota,
   and lists the published paid tiers for later. No card required.

## What turns billing back on

All of the following, in order, before any production Stripe secret:

1. Fresh Stripe **test** secret key (not an expired CLI key).
2. Real test-mode products and prices for Starter, Growth, Business CAD.
3. Full staging checkout: session opens, completes, webhook verifies,
   plan upgrades once, replay is idempotent.
4. Live-mode products and prices created. Live webhook registered against
   the production hostname.
5. Production secrets set with `wrangler secret put` (never `[vars]`).
6. Checkout and webhook routes restored to call `stripe.js`. Route C may
   be restored only by a dated kill-test amendment after paid billing is
   live.

## Consequences

- Kill test is A or B only until a further amendment.
- A live key that hits 100,000 in a month gets `402` / `plan_limit` with
  no upgrade path until billing returns. That is explicit on `/pricing/`.
- Integrators can plan against the published paid CAD prices without a
  contact-sales wedge.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
