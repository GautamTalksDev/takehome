//! Compile-fail: K4P must not accept AnnualTaxableIncome (spec §21.2).
//!
//! Yukon K4P uses the same employment-income newtype as federal K4.

fn main() {
    use netpay_core::decimal::{Money, Rate};
    use netpay_core::formulas::credits::{k4p, AnnualTaxableIncome};

    let rate = Rate::parse("0.0640").unwrap();
    let taxable = AnnualTaxableIncome::new(Money::parse("50000.00").unwrap());
    let cea = Money::parse("1501.00").unwrap();

    // Must not compile: AnnualTaxableIncome is not GrossEmploymentIncome.
    let _ = k4p(rate, taxable, cea);
}
