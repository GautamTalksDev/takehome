//! Compile-fail: Money must not convert from or into IEEE-754 floats.
//!
//! Driven by `tests/compile_fail.rs` under `--features compile-fail-tests`.

fn main() {
    use netpay_core::decimal::Money;

    // Each line must fail to compile.
    let _from_f64: Money = 1.0f64.into();
    let _from_f32: Money = 1.0f32.into();
    let _into_f64: f64 = Money::parse("1.00").unwrap().into();
}
