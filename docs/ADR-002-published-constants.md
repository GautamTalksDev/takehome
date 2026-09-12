# ADR-002: Published whole-dollar K

## Status

Accepted (M1)

## Context

T4127 computes annual federal (and provincial) tax as `(R × A) − K` on the
occupied bracket, rather than summing marginal slices. Algebraically K is

```text
K_i = Σ_{j=1..i} threshold_j × (rate_j − rate_{j−1})
```

That sum is not a whole number of dollars. CRA Table 8.1 (and the matching
provincial tables) publish K rounded to the nearest dollar via the same
half-up-to-the-dollar rule as TD1 claim indexing (`round_claim_to_dollar`).

The exact identity `(R × A) − K_exact = Σ marginal slices` therefore fails
against the published table by `|K_exact − K_published|`, which is at most
fifty cents per bracket. Crossing a threshold by one cent can move annual T3
by up to one dollar — the difference of two such residuals — rather than by
one cent.

An engine could store `K_exact` and match the algebra. PDOC uses the published
constants. T4032 uses the published constants. Every payroll system in Canada
uses the published constants. Netpay's claim is agreement with PDOC to the
cent.

## Decision

Store and use the published whole-dollar K. We are implementing a formula we
can prove is imprecise, on purpose, because fidelity to the published rule
beats fidelity to the algebra.

Test 28 remains the transcription canary: accumulate `K_exact` in full-precision
decimals, apply `round_claim_to_dollar`, and compare to the stored constant.
It already passes for FED and ON and must run on every jurisdiction added in
M2.

Tests 84 and 90 pin the residual rather than demanding it be zero. A mistyped
K still fails immediately; the intended imprecision is named and bounded.

## Consequences

- Annual T3/T4 is discontinuous by at most $1.00 at each bracket threshold.
  Measured 2026 jumps live in
  `crates/netpay-core/tests/vectors/bracket_discontinuity_2026.json`.
- At P = 52 the largest possible withholding jump is under two cents.
- PDOC exhibits the same discontinuity. Matching it is conformance, not a
  defect. See `CONFORMANCE.md` §Methodology.
