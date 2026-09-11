# Kill tests

If any of these go red, **stop shipping**. They are not coverage fillers; they
are the M0 correctness contract.

## 291.66 canary (truncation ≠ half-up)

`truncate_exemption_to_cent(3500/12) == 291.66`

CRA publishes **291.66** for the monthly CPP basic exemption. Half-up would
yield **291.67**. Getting this wrong taxes every monthly-paid employee in
Canada by one cent per pay on the exemption path.

Twin: `round_contribution_to_cent` on the same input must be **291.67**. The
pair proves the two code paths are not aliases.

## Float ban

`./scripts/float-ban.sh` must stay clean. IEEE-754 `f32` / `f64` must not
appear in executable `netpay-core` sources. Money and rates are lexical
decimals only (see [ADR-001](docs/ADR-001-decimal.md)).

## Property suites

The 10 000-case proptest suites in `rounding.rs` (CRA wording ≡ half-up,
idempotence, error bound, monotonicity, scale) must remain green. A failure
means the Chapter 2 rounding story is not what we thought.
