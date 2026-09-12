//! Annual taxable income A and F5 / F5A / F5B (T4127 Chapter 4 step 1).
//!
//! ```text
//! A  = [P × (I − F − F2 − F5A − U1)] − HD − F1     (negative → T = L, done)
//! F5 = C × (0.0100 / 0.0595) + C2
//! F5A = F5 × ((PI − B) / PI);  F5B = F5 × (B / PI)
//! ```

use crate::decimal::{DecimalError, Money, Rate, Ratio};
use crate::request::PayPeriod;
use crate::rounding::round_half_up_to_cent;
use thiserror::Error;

/// Inputs to the Option 1 annual taxable-income formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnualTaxableIncomeInputs {
    pub pay_period: PayPeriod,
    pub gross_pay: Money,
    pub rpp: Money,
    pub alimony_pre_1997: Money,
    pub f5a: Money,
    pub union_dues: Money,
    pub prescribed_zone: Money,
    pub annual_deductions: Money,
}

/// Annual-income formula failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AnnualIncomeError {
    #[error("pensionable earnings PI must be non-zero")]
    ZeroPensionableEarnings,
    #[error(transparent)]
    Decimal(#[from] DecimalError),
}

/// F5 = C × (first additional CPP rate / total CPP rate) + C2.
pub fn f5(
    c: Money,
    c2: Money,
    first_additional_rate: Rate,
    total_rate: Rate,
) -> Result<Money, AnnualIncomeError> {
    let ratio = Ratio::div(&first_additional_rate.to_string(), &total_rate.to_string())?;
    Ok(round_half_up_to_cent(
        c.checked_mul_ratio(ratio)?.checked_add(c2)?,
    ))
}

fn split_f5(f5: Money, pensionable: Money, numerator: Money) -> Result<Money, AnnualIncomeError> {
    if pensionable.is_zero() {
        return Err(AnnualIncomeError::ZeroPensionableEarnings);
    }
    Ok(round_half_up_to_cent(
        f5.checked_mul_ratio(numerator.checked_div(pensionable)?)?,
    ))
}

/// F5A, the regular-pay share of F5.
pub fn f5a(
    f5: Money,
    pensionable_earnings: Money,
    bonus: Money,
) -> Result<Money, AnnualIncomeError> {
    split_f5(
        f5,
        pensionable_earnings,
        pensionable_earnings.checked_sub(bonus)?,
    )
}

/// F5B, the bonus share of F5.
pub fn f5b(
    f5: Money,
    pensionable_earnings: Money,
    bonus: Money,
) -> Result<Money, AnnualIncomeError> {
    split_f5(f5, pensionable_earnings, bonus)
}

/// A = P × (I − F − F2 − F5A − U1) − HD − F1.
pub fn annual_taxable_income(
    inputs: &AnnualTaxableIncomeInputs,
) -> Result<Money, AnnualIncomeError> {
    let per_period = inputs
        .gross_pay
        .checked_sub(inputs.rpp)?
        .checked_sub(inputs.alimony_pre_1997)?
        .checked_sub(inputs.f5a)?
        .checked_sub(inputs.union_dues)?;
    let p = Money::parse(&inputs.pay_period.get().to_string())?;
    Ok(per_period
        .checked_mul(p)?
        .checked_sub(inputs.prescribed_zone)?
        .checked_sub(inputs.annual_deductions)?)
}

#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Rate};
    use crate::request::PayPeriod;
    use crate::rounding::round_half_up_to_cent;
    use crate::rules::schema::CppParams;

    use super::{
        annual_taxable_income, f5, f5a, f5b, AnnualIncomeError, AnnualTaxableIncomeInputs,
    };

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    fn cpp_2026() -> CppParams {
        CppParams {
            ympe: money("74600.00"),
            yampe: money("85000.00"),
            basic_exemption: money("3500.00"),
            total_rate: rate("0.0595"),
            total_max: money("4230.45"),
            base_rate: rate("0.0495"),
            base_max: money("3519.45"),
            first_additional_rate: rate("0.0100"),
            first_additional_max: money("711.00"),
            second_additional_rate: rate("0.0400"),
            second_additional_max: money("416.00"),
        }
    }

    // 112. F5 with C = 204.25, C2 = 0 → 34.33 (CRA §6.13). Same figure as
    // decimal.rs test 13, asserted through the real `f5` function.
    #[test]
    fn f5_cra_6_13_worked_example() {
        let cpp = cpp_2026();
        let got = f5(
            money("204.25"),
            money("0.00"),
            cpp.first_additional_rate,
            cpp.total_rate,
        )
        .unwrap();
        assert_eq!(got, money("34.33"));
        // Prove we did not just hard-code: the ratio path matches decimal.rs §6.13.
        let ratio = crate::decimal::Ratio::div("0.0100", "0.0595").unwrap();
        let via_raw = round_half_up_to_cent(money("204.25") * ratio);
        assert_eq!(got, via_raw);
    }

    /// 113. F5 = 0 when C = 0 and C2 = 0. PI = 0 → F5A/F5B return Err (no panic).
    #[test]
    fn f5_zero_when_c_and_c2_zero_and_pi_zero_is_err() {
        let cpp = cpp_2026();
        assert_eq!(
            f5(
                money("0.00"),
                money("0.00"),
                cpp.first_additional_rate,
                cpp.total_rate,
            )
            .unwrap(),
            money("0.00")
        );
        let err = f5a(money("34.33"), money("0.00"), money("0.00")).unwrap_err();
        assert!(
            matches!(err, AnnualIncomeError::ZeroPensionableEarnings),
            "got {err:?}"
        );
        let err_b = f5b(money("34.33"), money("0.00"), money("2500.00")).unwrap_err();
        assert!(
            matches!(err_b, AnnualIncomeError::ZeroPensionableEarnings),
            "got {err_b:?}"
        );
    }

    /// F5A / F5B split for the CRA §6.13 bonus numbers (PI = 3500, B = 2500).
    #[test]
    fn f5a_f5b_split_matches_cra_bonus_ratios() {
        let f5_val = money("34.33");
        let pi = money("3500.00");
        let b = money("2500.00");
        // 34.33 × (1000/3500) and 34.33 × (2500/3500)
        assert_eq!(f5a(f5_val, pi, b).unwrap(), money("9.81"));
        assert_eq!(f5b(f5_val, pi, b).unwrap(), money("24.52"));
    }

    /// Plain weekly: no bonus → B = 0 → F5A = F5, F5B = 0.
    #[test]
    fn f5a_equals_f5_when_bonus_is_zero() {
        let f5_val = money("9.33");
        assert_eq!(
            f5a(f5_val, money("1000.00"), money("0.00")).unwrap(),
            f5_val
        );
        assert_eq!(
            f5b(f5_val, money("1000.00"), money("0.00")).unwrap(),
            money("0.00")
        );
    }

    /// A = P × (I − F5A) when other deductions are zero.
    #[test]
    fn annual_taxable_income_plain_weekly() {
        let a = annual_taxable_income(&AnnualTaxableIncomeInputs {
            pay_period: PayPeriod::new(52).unwrap(),
            gross_pay: money("1000.00"),
            rpp: money("0.00"),
            alimony_pre_1997: money("0.00"),
            f5a: money("9.33"),
            union_dues: money("0.00"),
            prescribed_zone: money("0.00"),
            annual_deductions: money("0.00"),
        })
        .unwrap();
        assert_eq!(a, money("51514.84")); // 52 × (1000 − 9.33)
    }
}
