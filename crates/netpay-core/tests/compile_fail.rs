//! Compile-fail suite for netpay-core API negatives.
//!
//! Gated by `--features compile-fail-tests` (CI uses `--all-features`).

#![cfg(feature = "compile-fail-tests")]

#[test]
fn money_has_no_float_conversions() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/no_float.rs");
}

#[test]
fn k4_rejects_annual_taxable_income() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/k4_rejects_annual_taxable_income.rs");
}
