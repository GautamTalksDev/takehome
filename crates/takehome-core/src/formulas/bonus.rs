//! Bonuses, retroactive pay, and other non-periodic payments (T4127 Chapter 4 §6.13).
//!
//! Regular method (PDOC default) and optional year-to-date method. Each
//! bracketed group floors at zero independently. F5A and F5B are computed once
//! and reused for the with-bonus and without-bonus passes.

use crate::decimal::{DecimalError, Money, Rate};
use crate::request::{BonusMethod, PayPeriod, Province};
use crate::rounding::{round_half_up_to_cent, round_tax_to_cent};
use crate::rules::schema::{CppParams, EiParams};
use thiserror::Error;

/// Failure in the Chapter 4 bonus / retro formulas.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BonusError {
    #[error(transparent)]
    Decimal(#[from] DecimalError),
}

/// One group of the Chapter 4 bonus A formula. Each group floors at zero
/// independently (`*` / `**` notes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonusIncomeGroups {
    pub regular: Money,
    pub current_non_periodic: Money,
    pub prior_non_periodic: Money,
}

impl BonusIncomeGroups {
    /// A with the non-periodic payment payable now (Step 1).
    pub fn a_with(self) -> Result<Money, BonusError> {
        Ok(self
            .regular
            .checked_add(self.current_non_periodic)?
            .checked_add(self.prior_non_periodic)?)
    }

    /// A without the non-periodic payment payable now (Step 2).
    pub fn a_without(self) -> Result<Money, BonusError> {
        Ok(self.regular.checked_add(self.prior_non_periodic)?)
    }
}

/// Inputs shared by both bonus methods after F5A / F5B are known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonusIncomeInputs {
    pub pay_period: PayPeriod,
    pub periods_remaining: u16,
    pub periodic_income: Money,
    pub rpp: Money,
    pub alimony_pre_1997: Money,
    pub f5a: Money,
    pub union_dues: Money,
    pub prescribed_zone: Money,
    pub annual_deductions: Money,
    pub current_non_periodic: Money,
    pub bonus_rrsp: Money,
    pub f5b: Money,
    pub prior_non_periodic: Money,
    pub prior_bonus_rrsp: Money,
    pub f5b_ytd: Money,
    pub ytd_periodic_income: Money,
    pub ytd_rpp: Money,
    pub ytd_f5a: Money,
    pub ytd_union_dues: Money,
}

/// Shortcut threshold: if A with the bonus is at or below this, withhold a
/// flat rate on the bonus instead of the two-pass (T4127 Chapter 4 note).
pub fn bonus_shortcut_threshold() -> Result<Money, BonusError> {
    Ok(Money::parse("5000.00")?)
}

/// 15% outside Quebec; 10% in Quebec (T4127 Chapter 4 bonus note).
pub fn bonus_shortcut_rate(province: Province) -> Result<Rate, BonusError> {
    Ok(Rate::parse(if province == Province::Qc {
        "0.10"
    } else {
        "0.15"
    })?)
}

/// True when annual taxable income with the bonus is at or below $5,000.
pub fn bonus_shortcut_applies(a_with: Money) -> Result<bool, BonusError> {
    Ok(a_with <= bonus_shortcut_threshold()?)
}

/// Flat tax on the bonus when the $5,000 shortcut applies.
pub fn bonus_shortcut_tax(bonus: Money, province: Province) -> Result<Money, BonusError> {
    let raw = bonus.checked_mul_rate(bonus_shortcut_rate(province)?)?;
    Ok(round_tax_to_cent(raw.checked_div(Money::parse("1")?)?))
}

/// TB = max(0, (T1+T2)_with − (T1+T2)_without). Never negative.
pub fn tax_on_bonus(
    t1_with: Money,
    t2_with: Money,
    t1_without: Money,
    t2_without: Money,
) -> Result<Money, BonusError> {
    let with = t1_with.checked_add(t2_with)?;
    let without = t1_without.checked_add(t2_without)?;
    Ok(with.checked_sub(without)?.floor_at_zero())
}

fn floor_group(value: Money) -> Money {
    value.floor_at_zero()
}

/// Regular-method groups. T4127 Chapter 4, regular bonus calculation.
pub fn regular_bonus_groups(inputs: &BonusIncomeInputs) -> Result<BonusIncomeGroups, BonusError> {
    let p = Money::parse(&inputs.pay_period.get().to_string())?;
    let per_period = inputs
        .periodic_income
        .checked_sub(inputs.rpp)?
        .checked_sub(inputs.alimony_pre_1997)?
        .checked_sub(inputs.f5a)?
        .checked_sub(inputs.union_dues)?;
    let regular = floor_group(
        per_period
            .checked_mul(p)?
            .checked_sub(inputs.prescribed_zone)?
            .checked_sub(inputs.annual_deductions)?,
    );
    Ok(BonusIncomeGroups {
        regular,
        current_non_periodic: floor_group(
            inputs
                .current_non_periodic
                .checked_sub(inputs.bonus_rrsp)?
                .checked_sub(inputs.f5b)?,
        ),
        prior_non_periodic: floor_group(
            inputs
                .prior_non_periodic
                .checked_sub(inputs.prior_bonus_rrsp)?
                .checked_sub(inputs.f5b_ytd)?,
        ),
    })
}

/// Year-to-date method groups. T4127 Chapter 4, optional YTD bonus calculation.
pub fn year_to_date_bonus_groups(
    inputs: &BonusIncomeInputs,
) -> Result<BonusIncomeGroups, BonusError> {
    let pr = Money::parse(&inputs.periods_remaining.to_string())?;
    let ytd_periodic = inputs
        .ytd_periodic_income
        .checked_sub(inputs.ytd_rpp)?
        .checked_sub(inputs.ytd_f5a)?
        .checked_sub(inputs.ytd_union_dues)?;
    let rest = inputs
        .periodic_income
        .checked_sub(inputs.rpp)?
        .checked_sub(inputs.alimony_pre_1997)?
        .checked_sub(inputs.f5a)?
        .checked_sub(inputs.union_dues)?
        .checked_mul(pr)?;
    let regular = floor_group(
        ytd_periodic
            .checked_add(rest)?
            .checked_sub(inputs.annual_deductions)?
            .checked_sub(inputs.prescribed_zone)?,
    );
    Ok(BonusIncomeGroups {
        regular,
        current_non_periodic: floor_group(
            inputs
                .current_non_periodic
                .checked_sub(inputs.bonus_rrsp)?
                .checked_sub(inputs.f5b)?,
        ),
        prior_non_periodic: floor_group(
            inputs
                .prior_non_periodic
                .checked_sub(inputs.prior_bonus_rrsp)?
                .checked_sub(inputs.f5b_ytd)?,
        ),
    })
}

/// Groups for the selected method.
pub fn bonus_income_groups(
    method: BonusMethod,
    inputs: &BonusIncomeInputs,
) -> Result<BonusIncomeGroups, BonusError> {
    match method {
        BonusMethod::Regular => regular_bonus_groups(inputs),
        BonusMethod::YearToDate => year_to_date_bonus_groups(inputs),
    }
}

fn integer_money(value: u16) -> Result<Money, BonusError> {
    Ok(Money::parse(&value.to_string())?)
}

/// CPP / EI pieces used in the Chapter 4 bonus K2 example: annualize the
/// regular period only; add current and prior non-periodic amounts once.
#[allow(clippy::too_many_arguments)]
pub fn k2_non_periodic(
    lowest_rate: Rate,
    pay_periods: PayPeriod,
    c_regular: Money,
    c_current_bonus: Money,
    c_prior_bonus: Money,
    ei_regular: Money,
    ei_current_bonus: Money,
    ei_prior_bonus: Money,
    include_current_bonus: bool,
    cpp: &CppParams,
    ei: &EiParams,
) -> Result<Money, BonusError> {
    let p = integer_money(pay_periods.get())?;
    let ratio =
        crate::decimal::Ratio::div(&cpp.base_rate.to_string(), &cpp.total_rate.to_string())?;
    let c_current = if include_current_bonus {
        c_current_bonus
    } else {
        Money::ZERO
    };
    let ei_current = if include_current_bonus {
        ei_current_bonus
    } else {
        Money::ZERO
    };
    let cpp_regular = round_half_up_to_cent(c_regular.checked_mul_ratio(ratio)?);
    let cpp_current = round_half_up_to_cent(c_current.checked_mul_ratio(ratio)?);
    let cpp_prior = round_half_up_to_cent(c_prior_bonus.checked_mul_ratio(ratio)?);
    let cpp_base = cpp_regular
        .checked_mul(p)?
        .checked_add(cpp_current)?
        .checked_add(cpp_prior)?
        .min(cpp.base_max);
    let ei_base = ei_regular
        .checked_mul(p)?
        .checked_add(ei_current)?
        .checked_add(ei_prior_bonus)?
        .min(ei.employee_max);
    let credit_base = cpp_base.checked_add(ei_base)?;
    let raw = credit_base.checked_mul_rate(lowest_rate)?;
    Ok(round_tax_to_cent(raw.checked_div(Money::parse("1")?)?))
}

/// CPP or EI on a non-periodic amount with no basic exemption (T4127 bonus K2).
pub fn contribution_on_amount(amount: Money, rate: Rate) -> Result<Money, BonusError> {
    if amount.is_zero() || amount.is_negative() {
        return Ok(Money::ZERO);
    }
    Ok(crate::rounding::round_contribution_to_cent(
        amount
            .checked_mul_rate(rate)?
            .checked_div(Money::parse("1")?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        bonus_income_groups, bonus_shortcut_applies, bonus_shortcut_tax, contribution_on_amount,
        k2_non_periodic, regular_bonus_groups, tax_on_bonus, year_to_date_bonus_groups,
        BonusIncomeInputs,
    };
    use crate::decimal::{Money, Rate};
    use crate::request::{BonusMethod, PayPeriod, Province};
    use crate::rules::schema::{CppParams, EiParams};
    use proptest::prelude::*;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }
    fn p52() -> PayPeriod {
        PayPeriod::new(52).unwrap()
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

    fn cra_regular_inputs(f5a: Money, f5b: Money) -> BonusIncomeInputs {
        BonusIncomeInputs {
            pay_period: p52(),
            periods_remaining: 23,
            periodic_income: money("1000.00"),
            rpp: money("0.00"),
            alimony_pre_1997: money("0.00"),
            f5a,
            union_dues: money("0.00"),
            prescribed_zone: money("0.00"),
            annual_deductions: money("0.00"),
            current_non_periodic: money("2500.00"),
            bonus_rrsp: money("0.00"),
            f5b,
            prior_non_periodic: money("1500.00"),
            prior_bonus_rrsp: money("0.00"),
            f5b_ytd: money("14.60"),
            ytd_periodic_income: money("0.00"),
            ytd_rpp: money("0.00"),
            ytd_f5a: money("0.00"),
            ytd_union_dues: money("0.00"),
        }
    }

    /// 30. CRA worked example — A with / without from the published F5A / F5B.
    #[test]
    fn cra_regular_bonus_a_with_and_without() {
        let groups =
            regular_bonus_groups(&cra_regular_inputs(money("9.81"), money("24.52"))).unwrap();
        assert_eq!(groups.a_with().unwrap(), money("55450.76"));
        assert_eq!(groups.a_without().unwrap(), money("52975.28"));
    }

    /// 30. K2 with / without matches the published example (annualize I; add B and B1 once).
    #[test]
    fn cra_regular_bonus_k2_with_and_without() {
        let cpp = cpp_2026();
        let ei = ei_2026();
        let with = k2_non_periodic(
            rate("0.14"),
            p52(),
            money("55.50"),
            money("148.75"),
            money("89.25"),
            money("16.30"),
            money("40.75"),
            money("24.45"),
            true,
            &cpp,
            &ei,
        )
        .unwrap();
        let without = k2_non_periodic(
            rate("0.14"),
            p52(),
            money("55.50"),
            money("148.75"),
            money("89.25"),
            money("16.30"),
            money("40.75"),
            money("24.45"),
            false,
            &cpp,
            &ei,
        )
        .unwrap();
        assert_eq!(with, money("491.63"));
        assert_eq!(without, money("468.60"));
    }

    /// 31. Each bracketed group floors at zero independently.
    #[test]
    fn each_bracketed_group_floors_at_zero_independently() {
        let mut inputs = cra_regular_inputs(money("9.81"), money("24.52"));
        inputs.prescribed_zone = money("999999.00");
        inputs.bonus_rrsp = money("4000.00");
        inputs.f5b_ytd = money("5000.00");
        let groups = regular_bonus_groups(&inputs).unwrap();
        assert_eq!(groups.regular, money("0.00"));
        assert_eq!(groups.current_non_periodic, money("0.00"));
        assert_eq!(groups.prior_non_periodic, money("0.00"));
        assert_eq!(groups.a_with().unwrap(), money("0.00"));

        let mut only_regular_negative = cra_regular_inputs(money("9.81"), money("24.52"));
        only_regular_negative.annual_deductions = money("999999.00");
        let g = regular_bonus_groups(&only_regular_negative).unwrap();
        assert_eq!(g.regular, money("0.00"));
        assert_eq!(g.current_non_periodic, money("2475.48"));
        assert_eq!(g.prior_non_periodic, money("1485.40"));
        assert_eq!(g.a_with().unwrap(), money("3960.88"));
        assert_eq!(g.a_without().unwrap(), money("1485.40"));
    }

    /// 32. $5,000 shortcut boundary is on A with the bonus, inclusive.
    #[test]
    fn five_thousand_shortcut_boundary() {
        assert!(bonus_shortcut_applies(money("5000.00")).unwrap());
        assert!(bonus_shortcut_applies(money("4999.99")).unwrap());
        assert!(!bonus_shortcut_applies(money("5000.01")).unwrap());
        assert_eq!(
            bonus_shortcut_tax(money("5000.00"), Province::On).unwrap(),
            money("750.00")
        );
        assert_eq!(
            bonus_shortcut_tax(money("4999.99"), Province::On).unwrap(),
            money("750.00")
        );
        assert_eq!(
            bonus_shortcut_tax(money("5000.00"), Province::Qc).unwrap(),
            money("500.00")
        );
        assert_ne!(
            bonus_shortcut_tax(money("5000.01"), Province::On).unwrap(),
            money("0.00")
        );
    }

    /// 33. F5A / F5B are arguments, not recomputed per pass — both A's use the same split.
    #[test]
    fn f5a_f5b_identical_on_both_passes() {
        let f5a = money("9.81");
        let f5b = money("24.52");
        let groups = regular_bonus_groups(&cra_regular_inputs(f5a, f5b)).unwrap();
        let expected_regular = money("1000.00")
            .checked_sub(f5a)
            .unwrap()
            .checked_mul(money("52"))
            .unwrap();
        assert_eq!(groups.regular, expected_regular);
        assert_eq!(
            groups.a_without().unwrap(),
            expected_regular
                .checked_add(money("1500.00"))
                .unwrap()
                .checked_sub(money("14.60"))
                .unwrap()
        );
        assert_eq!(
            groups.a_with().unwrap(),
            groups
                .a_without()
                .unwrap()
                .checked_add(money("2500.00"))
                .unwrap()
                .checked_sub(f5b)
                .unwrap()
        );
    }

    /// Optional YTD method: CRA Chapter 4 worked A with / without.
    #[test]
    fn cra_ytd_bonus_a_with_and_without() {
        let inputs = BonusIncomeInputs {
            pay_period: p52(),
            periods_remaining: 22,
            periodic_income: money("1100.00"),
            rpp: money("45.00"),
            alimony_pre_1997: money("0.00"),
            f5a: money("10.80"),
            union_dues: money("5.00"),
            prescribed_zone: money("0.00"),
            annual_deductions: money("0.00"),
            current_non_periodic: money("2500.00"),
            bonus_rrsp: money("1000.00"),
            f5b: money("24.53"),
            prior_non_periodic: money("1000.00"),
            prior_bonus_rrsp: money("0.00"),
            f5b_ytd: money("9.67"),
            ytd_periodic_income: money("30000.00"),
            ytd_rpp: money("1350.00"),
            ytd_f5a: money("280.24"),
            ytd_union_dues: money("150.00"),
        };
        let groups = year_to_date_bonus_groups(&inputs).unwrap();
        assert_eq!(groups.a_with().unwrap(), money("53547.96"));
        assert_eq!(groups.a_without().unwrap(), money("52072.49"));
        let via = bonus_income_groups(BonusMethod::YearToDate, &inputs).unwrap();
        assert_eq!(via.a_with().unwrap(), groups.a_with().unwrap());
    }

    /// 35. TB is never negative.
    #[test]
    fn tb_floors_at_zero_when_without_exceeds_with() {
        let tb = tax_on_bonus(
            money("100.00"),
            money("10.00"),
            money("200.00"),
            money("20.00"),
        )
        .unwrap();
        assert_eq!(tb, money("0.00"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10_000))]
        #[test]
        fn tb_never_negative(
            t1w in 0u64..=500_000,
            t2w in 0u64..=500_000,
            t1o in 0u64..=500_000,
            t2o in 0u64..=500_000,
        ) {
            fn from_cents(cents: u64) -> Money {
                Money::parse(&format!("{}.{:02}", cents / 100, cents % 100)).unwrap()
            }
            let tb = tax_on_bonus(
                from_cents(t1w),
                from_cents(t2w),
                from_cents(t1o),
                from_cents(t2o),
            )
            .unwrap();
            prop_assert!(tb >= Money::ZERO);
        }
    }

    #[test]
    fn contribution_on_bonus_matches_cra_k2_pieces() {
        let cpp = cpp_2026();
        let ei = ei_2026();
        assert_eq!(
            contribution_on_amount(money("2500.00"), cpp.total_rate).unwrap(),
            money("148.75")
        );
        assert_eq!(
            contribution_on_amount(money("1500.00"), cpp.total_rate).unwrap(),
            money("89.25")
        );
        assert_eq!(
            contribution_on_amount(money("2500.00"), ei.employee_rate).unwrap(),
            money("40.75")
        );
        assert_eq!(
            contribution_on_amount(money("1500.00"), ei.employee_rate).unwrap(),
            money("24.45")
        );
    }
}
