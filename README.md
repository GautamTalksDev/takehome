# Netpay

Rust implementation of CRA **T4127** payroll deduction formulas (`netpay-core`).

**[Kill tests](KILL-TEST.md)** — if these fail, do not ship.

## M0 status

Exact decimal substrate (`rust_decimal` newtype) and CRA Chapter 2 / 6 rounding
rules, tests first. See [ADR-001](docs/ADR-001-decimal.md).

## Determinism

`netpay-core` does no I/O, reads no clock, and opens no network connections.
Outputs depend only on inputs.

## Develop

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/float-ban.sh
cargo deny check
```
