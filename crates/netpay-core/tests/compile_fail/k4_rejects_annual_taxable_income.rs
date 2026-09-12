//! Compile-fail: K4 must not accept AnnualTaxableIncome (spec §21.2).
//!
//! Factor A (annual taxable income) and annual gross employment income are
//! different quantities. Confusing them is a silent wrong K4.

fn main() {
    use netpay_core::decimal::{Money, Rate};
    use netpay_core::formulas::credits::{k4, AnnualTaxableIncome};

    let rate = Rate::parse("0.1400").unwrap();
    let taxable = AnnualTaxableIncome::new(Money::parse("50000.00").unwrap());
    let cea = Money::parse("1501.00").unwrap();

    // Must not compile: AnnualTaxableIncome is not GrossEmploymentIncome.
    let _ = k4(rate, taxable, cea);
}
