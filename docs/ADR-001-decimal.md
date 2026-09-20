# ADR-001: Decimal substrate

**Who this is for:** Engine contributors choosing numeric types for T4127 (Payroll Deductions Formulas) money and rates.

**When you finish:** You know why we wrap `rust_decimal` and where rounding belongs.

## Status

Accepted (M0)

## Context

The M0 spec says "implement the fixed point decimal type." Hand-rolling a correctly-rounded arbitrary-precision decimal with exact division is a multi-day project with a long tail of subtle bugs, and it is not the product differentiator. The project had one week.

## Decision

Wrap `rust_decimal`; do not write a decimal type from scratch.

`rust_decimal` provides a 96-bit mantissa, base-10 arithmetic that is exact for add/sub/mul, uses explicit precision for div, and has a `FromStr` that parses lexical form directly. Its float conversions live behind features that stay off.

Wrap it in a newtype so that the only constructors available in this crate are string-based. That gives the structural guarantee the takehome-core constraints require: money and rate values never pass through IEEE-754 floats.

## Money scale vs display

`Money` carries **unbounded internal scale** (up to `rust_decimal`'s 28 to 29 significant digits). It must: formulas such as `0.0595 × (PI − 3500/P)` produce many-decimal intermediates that only round at the very end.

- **Parse** accepts more than two fractional digits (e.g. `"5.005"` is `Ok` with scale 3).
- **`to_string`** always emits exactly two fractional digits (pads when scale &lt; 2).
- **Cent enforcement** belongs in `rounding.rs` via named functions, not in the `Money` type. Display padding is not formula rounding; payable cents always go through an explicit rounder.

`Rate` and `Ratio` likewise retain scale; Chapter 6 ratios are never pre-rounded.

## Consequences

Where `rust_decimal` bites:

- **28 to 29 significant digit ceiling** is irrelevant; CRA formulas divide five-figure dollar amounts.
- **`checked_div` rounds to 28dp banker's-style internally** does not touch a CRA formula, because every CRA division is either (a) immediately fed into a named rounding function, or (b) a ratio like `0.0100/0.0595` that the spec explicitly says must never be rounded. For those, the full 28dp intermediate is kept.

If this choice turns out to be wrong in M2, replace the internals of one newtype and nothing else in the codebase changes. That is the whole reason for the newtype.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
