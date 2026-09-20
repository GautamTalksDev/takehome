//! Option 2 — tax formulas based on cumulative averaging (T4127 Chapter 5).
//!
//! S1 is a pair (total periods / current period number), never a binary float.
//! Weekly period 1 is `52/1`; period 2 is `52/2`.

use crate::decimal::{DecimalError, Money, Rate};
use crate::formulas::bonus::BonusIncomeGroups;
use crate::request::PayPeriod;
use crate::rounding::{round_contribution_to_cent, round_tax_to_cent};
use crate::rules::schema::{CppParams, EiParams};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use thiserror::Error;

#[cfg(test)]
mod tests {
    use super::{
        annual_taxable_income_option2, k2_option2, period_tax_option2, scale_by_s1,
        Option2IncomeInputs, S1,
    };
    use crate::decimal::{Money, Rate};
    use crate::formulas::cpp::cpp_contribution;
    use crate::formulas::credits::k2;
    use crate::formulas::ei::ei_premium;
    use crate::request::{K2Method, PayPeriod};
    use crate::rules::schema::{CppParams, EiParams};

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }
    fn p(n: u16) -> PayPeriod {
        PayPeriod::new(n).unwrap()
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

    fn ei_2026() -> EiParams {
        EiParams {
            max_insurable: money("68900.00"),
            employee_rate: rate("0.0163"),
            employee_max: money("1123.07"),
            employer_rate: rate("0.02282"),
            employer_max: money("1572.30"),
        }
    }

    fn blank_inputs(s1: S1, gross: Money) -> Option2IncomeInputs {
        Option2IncomeInputs {
            s1,
            gross_pay: gross,
            rpp: money("0.00"),
            alimony_pre_1997: money("0.00"),
            f5a: money("0.00"),
            union_dues: money("0.00"),
            prescribed_zone: money("0.00"),
            annual_deductions: money("0.00"),
            prior_non_periodic: money("0.00"),
            prior_bonus_rrsp: money("0.00"),
            f5b: money("0.00"),
        }
    }

    /// 38. S1 is a pair (total periods / current period number), not a float.
    ///     Weekly period 1 is 52/1, period 2 is 52/2. Assert it stays exact.
    #[test]
    fn s1_is_an_exact_pair_not_a_float() {
        let weekly = p(52);
        let period1 = S1::from_pay_progress(weekly, None).unwrap();
        assert_eq!(period1.total_periods(), 52);
        assert_eq!(period1.current_period(), 1);
        assert_eq!(period1.to_string(), "52/1");
        let encoded = serde_json::to_value(period1).unwrap();
        assert_eq!(encoded, serde_json::json!("52/1"));
        assert!(encoded.as_str().is_some(), "S1 must serialize as a string");
        assert!(!encoded.is_number(), "S1 must never be a JSON number");

        let period2 = S1::from_pay_progress(weekly, Some(1)).unwrap();
        assert_eq!(period2.total_periods(), 52);
        assert_eq!(period2.current_period(), 2);
        assert_eq!(period2.to_string(), "52/2");
        let encoded2 = serde_json::to_value(period2).unwrap();
        assert_eq!(encoded2, serde_json::json!("52/2"));
        assert!(!encoded2.is_number());
        // 52/2 × $1,000.00 is $26,000.00 exactly — not 26.0 as a float factor.
        assert_eq!(
            scale_by_s1(money("1000.00"), period2).unwrap(),
            money("26000.00")
        );

        // T4127 Table 5.1.
        assert_eq!(
            S1::from_pay_progress(p(26), None).unwrap().to_string(),
            "26/1"
        );
        assert_eq!(
            S1::from_pay_progress(p(26), Some(1)).unwrap().to_string(),
            "26/2"
        );
        assert_eq!(
            S1::from_pay_progress(p(24), None).unwrap().to_string(),
            "24/1"
        );
        assert_eq!(
            S1::from_pay_progress(p(12), None).unwrap().to_string(),
            "12/1"
        );
    }

    /// 39. A = [S1 × (I − F − F2 − F5A − U1)] + (B1 − F4 − F5B) − HD − F1, floor 0.
    #[test]
    fn option2_a_matches_chapter5_and_floors_at_zero() {
        // T4127 Chapter 5: ($20,000 + $500) × 26/21 = $25,380.95.
        let s1 = S1::new(26, 21).unwrap();
        let projected = scale_by_s1(money("20500.00"), s1).unwrap();
        assert_eq!(projected, money("25380.95"));
        let a = annual_taxable_income_option2(&blank_inputs(s1, money("20500.00"))).unwrap();
        assert_eq!(a, money("25380.95"));

        let with_b1 = annual_taxable_income_option2(&Option2IncomeInputs {
            prior_non_periodic: money("100.00"),
            prior_bonus_rrsp: money("10.00"),
            f5b: money("5.00"),
            prescribed_zone: money("20.00"),
            annual_deductions: money("15.00"),
            rpp: money("50.00"),
            alimony_pre_1997: money("0.00"),
            f5a: money("9.33"),
            union_dues: money("1.00"),
            ..blank_inputs(S1::new(52, 1).unwrap(), money("1000.00"))
        })
        .unwrap();
        // [52 × (1000 − 50 − 0 − 9.33 − 1)] + (100 − 10 − 5) − 20 − 15
        // = 52 × 939.67 + 85 − 35 = 48862.84 + 50 = 48912.84
        assert_eq!(with_b1, money("48912.84"));

        let floored = annual_taxable_income_option2(&Option2IncomeInputs {
            prescribed_zone: money("50000.00"),
            ..blank_inputs(S1::new(52, 1).unwrap(), money("10.00"))
        })
        .unwrap();
        assert_eq!(floored, money("0.00"));
        assert!(!floored.is_negative());
    }

    /// 40. T = [((T1 + T2 − M1) / S1) − M] + L; negative means T = L.
    #[test]
    fn option2_t_matches_chapter5_and_negative_is_l() {
        let s1 = S1::new(26, 21).unwrap();
        // T4127 Chapter 5 fictitious example: $3,560.17 / 26 × 21 = $2,875.52;
        // minus M $2,736.40 → $139.12.
        let t = period_tax_option2(
            money("3560.17"),
            money("0.00"),
            s1,
            money("2736.40"),
            money("0.00"),
            money("0.00"),
        )
        .unwrap();
        assert_eq!(t, money("139.12"));

        let with_m1 = period_tax_option2(
            money("2000.00"),
            money("1560.17"),
            s1,
            money("2736.40"),
            money("100.00"),
            money("0.00"),
        )
        .unwrap();
        // ((3560.17 − 100) / S1) − 2736.40 = 2794.75 − 2736.40 = 58.35
        assert_eq!(with_m1, money("58.35"));

        let l = money("5.00");
        let negative = period_tax_option2(
            money("100.00"),
            money("0.00"),
            s1,
            money("5000.00"),
            money("0.00"),
            l,
        )
        .unwrap();
        assert_eq!(negative, l);
    }

    /// 41. K2 under Option 2 uses the Chapter 5 form, not the Option 1 form.
    #[test]
    fn option2_k2_uses_chapter5_form_not_option1() {
        let s1 = S1::from_pay_progress(p(52), None).unwrap();
        let pe = money("1000.00");
        let k2_opt2 = k2_option2(
            rate("0.14"),
            s1,
            pe,
            pe,
            money("0.00"),
            false,
            &cpp_2026(),
            &ei_2026(),
        )
        .unwrap();
        // 0.14 × (0.0495 × (52×1000 − 3500) + 0.0163 × 52×1000)
        // = 0.14 × (2400.75 + 847.60) = 454.769 → 454.77
        assert_eq!(k2_opt2, money("454.77"));

        let c = cpp_contribution(pe, money("0.00"), p(52), 12, &cpp_2026(), false).unwrap();
        let ei = ei_premium(pe, money("0.00"), &ei_2026(), false).unwrap();
        let k2_opt1 = k2(
            rate("0.14"),
            p(52),
            c,
            money("0.00"),
            ei,
            money("0.00"),
            52,
            12,
            K2Method::PdocObserved,
            &cpp_2026(),
            &ei_2026(),
        )
        .unwrap();
        assert_ne!(
            k2_opt2, k2_opt1,
            "Option 2 K2 must not silently reuse the Option 1 (P × C) form"
        );
    }

    /// 43. S1 resets to period 1 when switching options mid-year or when an
    ///     employee starts mid-year (omit / zero `pay_periods_elapsed`).
    #[test]
    fn s1_resets_to_period_1_on_midyear_start_or_option_switch() {
        let weekly = p(52);
        // Mid-year start or switch: do not carry the Option 1 clock (52/27).
        let reset = S1::from_pay_progress(weekly, None).unwrap();
        assert_eq!(reset.to_string(), "52/1");
        let also_reset = S1::from_pay_progress(weekly, Some(0)).unwrap();
        assert_eq!(also_reset.to_string(), "52/1");
        // Continuing the Option 2 clock after 26 completed weeks is 52/27.
        let continued = S1::from_pay_progress(weekly, Some(26)).unwrap();
        assert_eq!(continued.to_string(), "52/27");
        assert_ne!(reset.to_string(), continued.to_string());
    }
}

/// Failure in the Chapter 5 Option 2 formulas.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Option2Error {
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    #[error("S1 current period {current} is invalid for {total} pay periods")]
    InvalidS1 { total: u16, current: u16 },
}

/// T4127 factor S1: total pay periods / current pay-period number.
///
/// T4127 Chapter 5. Serialized as `"52/1"`, never a JSON number or IEEE float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S1 {
    total_periods: u16,
    current_period: u16,
}

impl S1 {
    /// Construct a validated pair. Current must be in `1..=total`.
    pub fn new(total_periods: u16, current_period: u16) -> Result<Self, Option2Error> {
        if total_periods == 0 || current_period == 0 || current_period > total_periods {
            return Err(Option2Error::InvalidS1 {
                total: total_periods,
                current: current_period,
            });
        }
        Ok(Self {
            total_periods,
            current_period,
        })
    }

    /// Placeholder for an uncomputed breakdown (`"0/1"`). Not a legal S1.
    pub const fn zero_placeholder() -> Self {
        Self {
            total_periods: 0,
            current_period: 1,
        }
    }

    /// Option 1 annualization is the pair `P/1`.
    pub fn for_option1(pay_period: PayPeriod) -> Result<Self, Option2Error> {
        Self::new(pay_period.get(), 1)
    }

    /// Option 2 S1 from P and completed periods before this cheque.
    ///
    /// T4127 Chapter 5 (Special situations): when switching to Option 2 mid-year
    /// or when an employee starts mid-year, reset to period 1 by omitting
    /// `pay_periods_elapsed` (or passing 0) so S1 is `P/1` rather than `P/27`.
    pub fn from_pay_progress(
        pay_period: PayPeriod,
        pay_periods_elapsed: Option<u16>,
    ) -> Result<Self, Option2Error> {
        let total = pay_period.get();
        let current = match pay_periods_elapsed {
            None | Some(0) => 1,
            Some(elapsed) => elapsed.saturating_add(1).clamp(1, total),
        };
        Self::new(total, current)
    }

    /// Numerator: total pay periods (or the employee’s remaining periods).
    pub const fn total_periods(self) -> u16 {
        self.total_periods
    }

    /// Denominator: current pay-period number (1-based).
    pub const fn current_period(self) -> u16 {
        self.current_period
    }
}

impl fmt::Display for S1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.total_periods, self.current_period)
    }
}

impl Serialize for S1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for S1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let (total, current) = raw.split_once('/').ok_or_else(|| {
            serde::de::Error::custom("S1 must be serialized as total/current, e.g. 52/1")
        })?;
        let total_periods = total.parse().map_err(serde::de::Error::custom)?;
        let current_period = current.parse().map_err(serde::de::Error::custom)?;
        Ok(Self {
            total_periods,
            current_period,
        })
    }
}

fn integer_money(value: u16) -> Result<Money, Option2Error> {
    Ok(Money::parse(&value.to_string())?)
}

/// `amount × total / current`, then [`round_tax_to_cent`].
///
/// T4127 Chapter 5 — projected income / A using S1.
pub fn scale_by_s1(amount: Money, s1: S1) -> Result<Money, Option2Error> {
    let total = integer_money(s1.total_periods)?;
    let current = integer_money(s1.current_period)?;
    Ok(round_tax_to_cent(
        amount.checked_mul(total)?.checked_div(current)?,
    ))
}

/// `amount × current / total`, then [`round_tax_to_cent`].
///
/// T4127 Chapter 5 — `(T1 + T2 − M1) / S1`.
pub fn divide_by_s1(amount: Money, s1: S1) -> Result<Money, Option2Error> {
    let total = integer_money(s1.total_periods)?;
    let current = integer_money(s1.current_period)?;
    Ok(round_tax_to_cent(
        amount.checked_mul(current)?.checked_div(total)?,
    ))
}

/// Inputs for Option 2 annual taxable income A. `gross_pay` / `rpp` / `f5a` /
/// `union_dues` already include the matching YTD amounts (T4127 Chapter 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Option2IncomeInputs {
    pub s1: S1,
    pub gross_pay: Money,
    pub rpp: Money,
    pub alimony_pre_1997: Money,
    pub f5a: Money,
    pub union_dues: Money,
    pub prescribed_zone: Money,
    pub annual_deductions: Money,
    pub prior_non_periodic: Money,
    pub prior_bonus_rrsp: Money,
    pub f5b: Money,
}

/// A = [S1 × (I − F − F2 − F5A − U1)] + (B1 − F4 − F5B) − HD − F1.
///
/// T4127 Chapter 5 — Formula to calculate annual taxable income (A).
/// If the result is negative, A = $0.
pub fn annual_taxable_income_option2(inputs: &Option2IncomeInputs) -> Result<Money, Option2Error> {
    let periodic = inputs
        .gross_pay
        .checked_sub(inputs.rpp)?
        .checked_sub(inputs.alimony_pre_1997)?
        .checked_sub(inputs.f5a)?
        .checked_sub(inputs.union_dues)?;
    let projected = scale_by_s1(periodic, inputs.s1)?;
    let prior = inputs
        .prior_non_periodic
        .checked_sub(inputs.prior_bonus_rrsp)?
        .checked_sub(inputs.f5b)?;
    Ok(projected
        .checked_add(prior)?
        .checked_sub(inputs.prescribed_zone)?
        .checked_sub(inputs.annual_deductions)?
        .floor_at_zero())
}

/// Option 2 bonus / retro groups (T4127 Chapter 5, TB Step 1 / Step 2).
///
/// Regular group floors independently after S1 × periodic − HD − F1.
pub fn option2_bonus_groups(
    inputs: &Option2IncomeInputs,
    current_non_periodic: Money,
    bonus_rrsp: Money,
    f5b_current: Money,
) -> Result<BonusIncomeGroups, Option2Error> {
    let periodic = inputs
        .gross_pay
        .checked_sub(inputs.rpp)?
        .checked_sub(inputs.alimony_pre_1997)?
        .checked_sub(inputs.f5a)?
        .checked_sub(inputs.union_dues)?;
    let regular = scale_by_s1(periodic, inputs.s1)?
        .checked_sub(inputs.prescribed_zone)?
        .checked_sub(inputs.annual_deductions)?
        .floor_at_zero();
    Ok(BonusIncomeGroups {
        regular,
        current_non_periodic: current_non_periodic
            .checked_sub(bonus_rrsp)?
            .checked_sub(f5b_current)?
            .floor_at_zero(),
        prior_non_periodic: inputs
            .prior_non_periodic
            .checked_sub(inputs.prior_bonus_rrsp)?
            .checked_sub(inputs.f5b)?
            .floor_at_zero(),
    })
}

/// T = [((T1 + T2 − M1) / S1) − M]* + L.
///
/// T4127 Chapter 5 — estimated federal and provincial tax for the pay period.
/// If the starred result is negative, T = L.
pub fn period_tax_option2(
    t1: Money,
    t2: Money,
    s1: S1,
    m: Money,
    m1: Money,
    additional_tax: Money,
) -> Result<Money, Option2Error> {
    let annual = t1.checked_add(t2)?.checked_sub(m1)?;
    let proportional = divide_by_s1(annual, s1)?;
    let after_m = proportional.checked_sub(m)?;
    if after_m.is_negative() {
        Ok(additional_tax)
    } else {
        Ok(after_m.checked_add(additional_tax)?)
    }
}

/// K2 = [(lowest rate × (base rate × ((S1 × PE) + B1 − $3,500)*, max base_max))
///     + (lowest rate × (EI rate × ((S1 × IE) + B1), max EI max))].
///
/// T4127 Chapter 5 — Formula to calculate basic federal tax (T3), factor K2.
/// The Option 1 form (`P × C × base/total`) is not used.
#[allow(clippy::too_many_arguments)] // T4127 K2 Option 2 inputs are inherently many.
pub fn k2_option2(
    lowest_rate: Rate,
    s1: S1,
    pensionable_earnings: Money,
    insurable_earnings: Money,
    prior_non_periodic: Money,
    cpp_exempt: bool,
    cpp: &CppParams,
    ei: &EiParams,
) -> Result<Money, Option2Error> {
    let one = Money::parse("1")?;
    let cpp_base = if cpp_exempt {
        Money::ZERO
    } else {
        let projected_pe = scale_by_s1(pensionable_earnings, s1)?;
        let cpp_income = projected_pe
            .checked_add(prior_non_periodic)?
            .checked_sub(cpp.basic_exemption)?
            .floor_at_zero();
        round_contribution_to_cent(
            cpp_income
                .checked_mul_rate(cpp.base_rate)?
                .checked_div(one)?,
        )
        .min(cpp.base_max)
    };
    let projected_ie = scale_by_s1(insurable_earnings, s1)?;
    let ei_income = projected_ie.checked_add(prior_non_periodic)?;
    let ei_base = round_contribution_to_cent(
        ei_income
            .checked_mul_rate(ei.employee_rate)?
            .checked_div(one)?,
    )
    .min(ei.employee_max);
    let credit_base = cpp_base.checked_add(ei_base)?;
    Ok(round_tax_to_cent(
        credit_base
            .checked_mul_rate(lowest_rate)?
            .checked_div(one)?,
    ))
}
