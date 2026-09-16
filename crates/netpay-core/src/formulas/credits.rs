//! Non-refundable tax credits K1–K4 / K1P / K2P (T4127 Chapter 4 / §6.3).
//!
//! Trap §21.2: [`k4`] and [`k4p`] take [`GrossEmploymentIncome`], not [`AnnualTaxableIncome`].
//! Trap §21.1: [`K2Method`] selects the CPP-credit base. Default
//! [`K2Method::PdocObserved`] follows PDOC (`max(P×C×ratio, D×ratio)` capped)
//! and does **not** force `base_max` in the reaching period.
//! [`K2Method::T4127Literal`] is the document's reading of that sentence.

use std::cmp::Ordering;

use crate::decimal::{Money, Rate, Ratio};
use crate::formulas::cpp::times_pm_over_twelve;
use crate::request::{K2Method, PayPeriod};
use crate::rounding::round_tax_to_cent;
use crate::rules::schema::{CppParams, EiParams};
use thiserror::Error;

#[cfg(test)]
mod tests {
    use super::{
        k1, k1p, k2, k2_cpp_annualized_base_before_max_rule, k3_adjusted, k4, k4p,
        AnnualTaxableIncome, GrossEmploymentIncome,
    };
    use crate::decimal::{Money, Rate, Ratio};
    use crate::request::{K2Method, PayPeriod};
    use crate::rounding::round_tax_to_cent;
    use crate::rules::schema::{CppParams, EiParams};
    use proptest::prelude::*;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    fn cpp_params() -> CppParams {
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

    fn ei_params() -> EiParams {
        EiParams {
            max_insurable: money("68900.00"),
            employee_rate: rate("0.0163"),
            employee_max: money("1123.07"),
            employer_rate: rate("0.02282"),
            employer_max: money("1572.30"),
        }
    }

    /// 69. k4 with employment income 50000 and CEA 1501 → 0.14 × 1501 = 210.14
    #[test]
    fn k4_uses_cea_when_employment_income_above_cea() {
        let got = k4(
            rate("0.1400"),
            GrossEmploymentIncome::new(money("50000.00")),
            money("1501.00"),
        )
        .unwrap();
        assert_eq!(got, money("210.14"));
    }

    /// 70. k4 with employment income 1000 (below CEA) → 0.14 × 1000 = 140.00
    #[test]
    fn k4_uses_employment_income_when_below_cea() {
        let got = k4(
            rate("0.1400"),
            GrossEmploymentIncome::new(money("1000.00")),
            money("1501.00"),
        )
        .unwrap();
        assert_eq!(got, money("140.00"));
    }

    // 71 lives in `tests/compile_fail/k4_rejects_annual_taxable_income.rs` (trybuild).

    /// Test 28 — K4P uses the same CEA cap as K4, Yukon lowest rate 0.064.
    #[test]
    fn k4p_lesser_of_employment_income_and_cea() {
        let cea = money("1501.00");
        let r = rate("0.0640");
        assert_eq!(
            k4p(r, GrossEmploymentIncome::new(money("50000.00")), cea).unwrap(),
            money("96.06")
        );
        assert_eq!(
            k4p(r, GrossEmploymentIncome::new(money("1000.00")), cea).unwrap(),
            money("64.00")
        );
    }

    // Test 28 compile-fail lives in `tests/compile_fail/k4p_rejects_annual_taxable_income.rs`.

    fn k2_call(
        method: K2Method,
        period_cpp: Money,
        ytd_cpp_before: Money,
        period_ei: Money,
        ytd_ei_before: Money,
        periods_remaining: u16,
    ) -> Money {
        k2(
            rate("0.1400"),
            PayPeriod::new(26).unwrap(),
            period_cpp,
            ytd_cpp_before,
            period_ei,
            ytd_ei_before,
            periods_remaining,
            12,
            method,
            &cpp_params(),
            &ei_params(),
        )
        .unwrap()
    }

    /// 72. Early in the year, all three K2 methods agree on a steady-salary case.
    #[test]
    fn k2_methods_agree_early_steady_salary() {
        let c = money("55.50");
        let ei = money("16.30");
        let pdoc = k2_call(
            K2Method::PdocObserved,
            c,
            money("0.00"),
            ei,
            money("0.00"),
            26,
        );
        let literal = k2_call(
            K2Method::T4127Literal,
            c,
            money("0.00"),
            ei,
            money("0.00"),
            26,
        );
        let ytd = k2_call(
            K2Method::YearToDate,
            c,
            money("0.00"),
            ei,
            money("0.00"),
            26,
        );
        assert_eq!(pdoc, ytd);
        assert_eq!(pdoc, literal);
    }

    /// 73. Poison case: CPP max reached in period 20 of 26.
    ///
    /// Re-specified to PDOC's observed rule (vector `on-biweekly-midyear-k2-max`).
    /// [`K2Method::PdocObserved`] credits `max(P×C×ratio, D×ratio)` capped at
    /// `base_max × PM/12` — **not** forcing `base_max` in the reaching period.
    /// [`K2Method::T4127Literal`] is the T4127 Chapter 3 reading that does force
    /// it. [`K2Method::YearToDate`] projects `D + PR×C`. All three produce
    /// different K2 on this case (EI from the vector so the YTD EI half diverges).
    #[test]
    fn k2_midyear_reaching_period_three_methods() {
        let p = PayPeriod::new(26).unwrap();
        let params = cpp_params();
        let ei = ei_params();
        let r = rate("0.1400");
        let c_period_20 = money("30.45");
        let d_after_20 = money("4230.45");
        let d_before_20 = d_after_20.checked_sub(c_period_20).unwrap();
        let cap = crate::formulas::cpp::times_pm_over_twelve(params.base_max, 12).unwrap();
        assert_eq!(cap.to_string(), "3519.45");

        let ratio = Ratio::div("0.0495", "0.0595").unwrap();
        let period_20_cpp_base = d_before_20.checked_mul_ratio(ratio).unwrap();
        let expected_period_20 = round_tax_to_cent(
            (Money::from_decimal(period_20_cpp_base.as_decimal()) * r)
                .checked_div(money("1"))
                .unwrap(),
        );
        let expected_after_max = round_tax_to_cent(
            (Money::from_decimal(cap.as_decimal()) * r)
                .checked_div(money("1"))
                .unwrap(),
        );

        for period in 20..=26 {
            let pr = 26 - period + 1;
            let (c, d_before, expected) = if period == 20 {
                (c_period_20, d_before_20, expected_period_20)
            } else {
                (money("0.00"), d_after_20, expected_after_max)
            };
            let got = k2(
                r,
                p,
                c,
                d_before,
                money("0.00"),
                money("0.00"),
                pr,
                12,
                K2Method::PdocObserved,
                &params,
                &ei,
            )
            .unwrap();
            assert_eq!(got, expected, "period {period}");
            if period == 20 {
                let raw =
                    k2_cpp_annualized_base_before_max_rule(p, c_period_20, 12, &params).unwrap();
                assert_eq!(
                    raw.cmp_ratio(&cap),
                    std::cmp::Ordering::Less,
                    "period 20 raw P×C×ratio {raw} must be below base_max {cap}"
                );
                assert_eq!(
                    period_20_cpp_base.cmp_ratio(&cap),
                    std::cmp::Ordering::Less,
                    "PDOC path uses D×ratio below base_max in the reaching period"
                );
            }
        }

        // Same mid-year reaching period as `on-biweekly-midyear-k2-max`
        // (C=30.45, D=4200, EI=40.75, D1=1000, PR=7).
        let c = c_period_20;
        let d = d_before_20;
        let period_ei = money("40.75");
        let d1 = money("1000.00");
        let pr = 7u16;

        // PdocObserved — vector `on-biweekly-midyear-k2-max` (D-007 inversion).
        let pdoc = k2_call(K2Method::PdocObserved, c, d, period_ei, d1, pr);
        // T4127Literal — T4127 Chapter 3, factor K2: force base_max in that pay period.
        let literal = k2_call(K2Method::T4127Literal, c, d, period_ei, d1, pr);
        // YearToDate — T4127 Chapter 4 YTD form: D + PR×C and D1 + PR×EI, capped.
        let ytd = k2_call(K2Method::YearToDate, c, d, period_ei, d1, pr);

        assert_eq!(pdoc, money("637.51"));
        assert_eq!(literal, money("641.05"));
        assert_eq!(ytd, money("649.95"));
        assert_ne!(
            pdoc, literal,
            "PDOC observed must differ from T4127 literal"
        );
        assert_ne!(pdoc, ytd, "PDOC observed must differ from YearToDate");
        assert_ne!(literal, ytd, "T4127 literal must differ from YearToDate");
    }

    /// 74. YearToDate on the same employee also yields base_max from period 20.
    #[test]
    fn k2_year_to_date_also_at_base_max_from_period_20() {
        let p = PayPeriod::new(26).unwrap();
        let params = cpp_params();
        let ei = ei_params();
        let r = rate("0.1400");
        let expected = round_tax_to_cent((money("3519.45") * r).checked_div(money("1")).unwrap());
        let c = money("30.45");
        let d_before = money("4200.00");
        let got = k2(
            r,
            p,
            c,
            d_before,
            money("0.00"),
            money("0.00"),
            7, // PR from period 20 inclusive through 26
            12,
            K2Method::YearToDate,
            &params,
            &ei,
        )
        .unwrap();
        assert_eq!(got, expected);
    }

    /// 73b. `base_max × PM/12` is a Ratio (PM=6 → 1759.725). Rounding that cap
    /// to 1759.72 or to 1759.73 each produces a different K2 than the unrounded
    /// path once an EI addend shifts the half-up boundary (0.10 vs 0.02).
    #[test]
    fn k2_prorated_base_max_is_ratio_not_rounded_money() {
        let params = cpp_params();
        let cap = crate::formulas::cpp::times_pm_over_twelve(params.base_max, 6).unwrap();
        assert_eq!(cap, Ratio::parse("1759.725").unwrap());
        assert_eq!(money("1759.72").cmp_ratio(&cap), std::cmp::Ordering::Less);
        assert_eq!(
            money("1759.73").cmp_ratio(&cap),
            std::cmp::Ordering::Greater
        );

        let p = PayPeriod::new(1).unwrap();
        let r = rate("0.1400");
        // Two EI addends: 0.10 makes the 1759.72 Money cap disagree; 0.02 makes
        // 1759.73 disagree. Together they show both roundings of the cap are
        // the wrong operation — pinning either literal would miss the other.
        for (period_ei, money_cap, label) in [
            (money("0.10"), money("1759.72"), "1759.72"),
            (money("0.02"), money("1759.73"), "1759.73"),
        ] {
            let got = k2(
                r,
                p,
                money("0.00"),
                money("4230.45"),
                period_ei,
                money("0.00"),
                1,
                6,
                K2Method::PdocObserved,
                &params,
                &ei_params(),
            )
            .unwrap();
            let unrounded = round_tax_to_cent(
                (Money::from_decimal(cap.as_decimal())
                    .checked_add(period_ei)
                    .unwrap()
                    * r)
                    .checked_div(money("1"))
                    .unwrap(),
            );
            let via_money = round_tax_to_cent(
                (money_cap.checked_add(period_ei).unwrap() * r)
                    .checked_div(money("1"))
                    .unwrap(),
            );
            assert_eq!(
                got, unrounded,
                "engine must keep the {label} cap as a Ratio"
            );
            assert_ne!(
                via_money, unrounded,
                "rounding the cap to {label} then ×0.14 must not match the Ratio path"
            );
        }
    }

    /// 75. PdocObserved and YearToDate disagree on a lumpy-income case.
    ///
    /// Default is [`K2Method::PdocObserved`] (vector `on-biweekly-midyear-k2-max`).
    /// YearToDate stays selectable via `k2_method`.
    #[test]
    fn k2_methods_disagree_on_lumpy_income() {
        let p = PayPeriod::new(26).unwrap();
        let params = cpp_params();
        let ei = ei_params();
        let r = rate("0.1400");
        // Lumpy: large C early, then small; YTD vs PdocObserved diverge.
        let c = money("200.00");
        let d_before = money("2500.00");
        let period_ei = money("40.00");
        let d1_before = money("400.00");
        let pr = 10u16;
        let pdoc = k2(
            r,
            p,
            c,
            d_before,
            period_ei,
            d1_before,
            pr,
            12,
            K2Method::PdocObserved,
            &params,
            &ei,
        )
        .unwrap();
        let ytd = k2(
            r,
            p,
            c,
            d_before,
            period_ei,
            d1_before,
            pr,
            12,
            K2Method::YearToDate,
            &params,
            &ei,
        )
        .unwrap();
        assert_ne!(pdoc, ytd);
        let _ = (pdoc, ytd);
    }

    /// 76. base/total ratio (0.0495/0.0595) from the rule set is a Ratio, never rounded.
    #[test]
    fn cpp_base_total_ratio_keeps_scale_at_least_twenty() {
        let ratio = Ratio::div("0.0495", "0.0595").unwrap();
        assert!(
            ratio.scale() >= 20 || ratio.significant_decimals() >= 20,
            "base/total ratio must retain high scale, got scale={} sig={}",
            ratio.scale(),
            ratio.significant_decimals()
        );
        let from_params = Ratio::div(
            &cpp_params().base_rate.to_string(),
            &cpp_params().total_rate.to_string(),
        )
        .unwrap();
        assert_eq!(ratio, from_params);
    }

    /// 77. K3 late-introduction: K3=1000, P=26, PR=13 → 2000.00
    #[test]
    fn k3_late_introduction_scales_by_p_over_pr() {
        let got = k3_adjusted(money("1000.00"), PayPeriod::new(26).unwrap(), 13).unwrap();
        assert_eq!(got, money("2000.00"));
    }

    /// 78. K1P for Ontario uses 0.0505, not 0.14.
    #[test]
    fn k1p_uses_provincial_lowest_rate_not_federal() {
        let tcp = money("12989.00");
        let k1p_on = k1p(rate("0.0505"), tcp).unwrap();
        let wrong_federal = k1(rate("0.1400"), tcp).unwrap();
        assert_eq!(k1p_on, money("655.94"));
        assert_eq!(wrong_federal, money("1818.46"));
        assert_ne!(
            k1p_on, wrong_federal,
            "using federal 0.14 for K1P would look plausible but be wrong"
        );
    }

    /// 78b. Bracket-4 trap: K1/K2/K4 (and K1P/K2P) must use `lowest_rate`, never
    /// bracket R. Federal bracket 4 is 0.2900 vs lowest 0.1400 (15 points).
    #[test]
    fn credits_use_lowest_rate_not_bracket_r_in_high_bracket() {
        let lowest = rate("0.1400");
        let bracket_r = rate("0.2900"); // federal bracket 4; 15 points above lowest
        assert_ne!(lowest, bracket_r);

        let tc = money("16452.00");
        let k1_ok = k1(lowest, tc).unwrap();
        let k1_wrong = k1(bracket_r, tc).unwrap();
        assert_eq!(k1_ok, money("2303.28"));
        assert_ne!(k1_ok, k1_wrong);

        let k4_ok = k4(
            lowest,
            GrossEmploymentIncome::new(money("200000.00")),
            money("1501.00"),
        )
        .unwrap();
        let k4_wrong = k4(
            bracket_r,
            GrossEmploymentIncome::new(money("200000.00")),
            money("1501.00"),
        )
        .unwrap();
        assert_eq!(k4_ok, money("210.14"));
        assert_ne!(k4_ok, k4_wrong);

        let p = PayPeriod::new(26).unwrap();
        let k2_ok = k2(
            lowest,
            p,
            money("55.50"),
            money("0.00"),
            money("16.30"),
            money("0.00"),
            26,
            12,
            K2Method::PdocObserved,
            &cpp_params(),
            &ei_params(),
        )
        .unwrap();
        let k2_wrong = k2(
            bracket_r,
            p,
            money("55.50"),
            money("0.00"),
            money("16.30"),
            money("0.00"),
            26,
            12,
            K2Method::PdocObserved,
            &cpp_params(),
            &ei_params(),
        )
        .unwrap();
        assert_ne!(k2_ok, k2_wrong);

        let tcp = money("12989.00");
        let k1p_ok = k1p(rate("0.0505"), tcp).unwrap();
        let k1p_wrong = k1p(rate("0.2010"), tcp).unwrap(); // ON top bracket vs lowest
        assert_ne!(k1p_ok, k1p_wrong);

        let k2p_ok = k2(
            rate("0.0505"),
            p,
            money("55.50"),
            money("0.00"),
            money("16.30"),
            money("0.00"),
            26,
            12,
            K2Method::PdocObserved,
            &cpp_params(),
            &ei_params(),
        )
        .unwrap();
        let k2p_wrong = k2(
            rate("0.2010"),
            p,
            money("55.50"),
            money("0.00"),
            money("16.30"),
            money("0.00"),
            26,
            12,
            K2Method::PdocObserved,
            &cpp_params(),
            &ei_params(),
        )
        .unwrap();
        assert_ne!(k2p_ok, k2p_wrong);
    }

    // 79. Property: every K is >= 0.
    proptest! {
        #[test]
        fn credits_never_negative(
            tc_cents in 0u32..=5_000_000,
            rate_bps in 1u16..=3300u16,
        ) {
            let tc = money(&format!("{}.{:02}", tc_cents / 100, tc_cents % 100));
            let r = rate(&format!("0.{:04}", rate_bps));
            let k = k1(r, tc).unwrap();
            prop_assert!(!k.is_negative());
            let k4v = k4(r, GrossEmploymentIncome::new(tc), money("1501.00")).unwrap();
            prop_assert!(!k4v.is_negative());
            let _ = AnnualTaxableIncome::new(tc); // type exists; not usable as k4 input
        }
    }

    // 80. Property: increasing TC never decreases K1.
    proptest! {
        #[test]
        fn k1_monotone_in_tc(a_cents in 0u32..=5_000_000, b_cents in 0u32..=5_000_000) {
            prop_assume!(a_cents <= b_cents);
            let r = rate("0.1400");
            let a = money(&format!("{}.{:02}", a_cents / 100, a_cents % 100));
            let b = money(&format!("{}.{:02}", b_cents / 100, b_cents % 100));
            prop_assert!(k1(r, a).unwrap() <= k1(r, b).unwrap());
        }
    }
}

/// Annual taxable income (factor A). Not employment income — see [`GrossEmploymentIncome`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AnnualTaxableIncome(Money);

impl AnnualTaxableIncome {
    /// Wrap a computed factor A.
    pub fn new(amount: Money) -> Self {
        Self(amount)
    }

    /// Inner amount.
    pub fn amount(self) -> Money {
        self.0
    }
}

/// Annual gross income from office or employment (K4 / CEA base). Spec §21.2 —
/// the other meaning of "A". Cannot be built from [`AnnualTaxableIncome`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GrossEmploymentIncome(Money);

impl GrossEmploymentIncome {
    /// Wrap box-14-style employment income for K4.
    pub fn new(amount: Money) -> Self {
        Self(amount)
    }

    /// Inner amount.
    pub fn amount(self) -> Money {
        self.0
    }
}

/// Credit-formula failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CreditsError {
    #[error("credit arithmetic: {0}")]
    Decimal(String),
    #[error("invalid periods remaining PR={got}")]
    InvalidPeriodsRemaining { got: u16 },
}

fn decimal(error: impl std::fmt::Display) -> CreditsError {
    CreditsError::Decimal(error.to_string())
}

fn integer_money(value: u16) -> Result<Money, CreditsError> {
    Money::parse(&value.to_string()).map_err(decimal)
}

fn rounded_rate_product(amount: Money, rate: Rate) -> Result<Money, CreditsError> {
    let raw = amount.checked_mul_rate(rate).map_err(decimal)?;
    Ok(round_tax_to_cent(
        raw.checked_div(Money::parse("1").map_err(decimal)?)
            .map_err(decimal)?,
    ))
}

/// K1 = lowest federal rate × TC (T4127 Chapter 4).
pub fn k1(rate: Rate, tc: Money) -> Result<Money, CreditsError> {
    rounded_rate_product(tc.floor_at_zero(), rate)
}

/// K1P = lowest provincial rate × TCP (T4127 Chapter 4).
pub fn k1p(rate: Rate, tcp: Money) -> Result<Money, CreditsError> {
    rounded_rate_product(tcp.floor_at_zero(), rate)
}

/// K4 = lesser of (rate × annual gross employment income) and (rate × CEA).
///
/// Spec §21.2 — `annual_gross_employment_income` is [`GrossEmploymentIncome`],
/// never [`AnnualTaxableIncome`].
pub fn k4(
    rate: Rate,
    annual_gross_employment_income: GrossEmploymentIncome,
    cea: Money,
) -> Result<Money, CreditsError> {
    rounded_rate_product(
        annual_gross_employment_income
            .amount()
            .floor_at_zero()
            .min(cea.floor_at_zero()),
        rate,
    )
}

/// Yukon K4P — same formula as [`k4`], provincial lowest rate.
///
/// Spec §21.2 applies here too: employment income, not factor A.
pub fn k4p(
    provincial_lowest_rate: Rate,
    annual_gross_employment_income: GrossEmploymentIncome,
    cea: Money,
) -> Result<Money, CreditsError> {
    k4(provincial_lowest_rate, annual_gross_employment_income, cea)
}

/// K3 late-introduction adjustment: `(P × K3) / PR` (T4127 Chapter 4).
pub fn k3_adjusted(
    k3: Money,
    pay_periods: PayPeriod,
    periods_remaining: u16,
) -> Result<Money, CreditsError> {
    if periods_remaining == 0 || periods_remaining > pay_periods.get() {
        return Err(CreditsError::InvalidPeriodsRemaining {
            got: periods_remaining,
        });
    }
    let numerator = k3
        .checked_mul(integer_money(pay_periods.get())?)
        .map_err(decimal)?;
    let ratio = numerator
        .checked_div(integer_money(periods_remaining)?)
        .map_err(decimal)?;
    Ok(round_tax_to_cent(ratio))
}

/// K2 / K2P — CPP base + EI tax credits at `lowest_rate`.
///
/// [`K2Method::PdocObserved`] (default; PDOC-confirmed): CPP credit base is
/// `min(base_max×PM/12, max(P×C×ratio, D×ratio))` with EI `min(P×EI, EI_max)`.
/// That is: take the greater of the period-annualized figure and the YTD base
/// figure, capped at the prorated base maximum — **without** forcing
/// `base_max` in the pay period where YTD first reaches the annual CPP max.
///
/// [`K2Method::T4127Literal`] (T4127 Chapter 3, factor K2): the Chapter 4
/// formula `min(P×C×ratio, base_max×PM/12)`, but if this period's YTD CPP
/// reaches the annual maximum, force `base_max` (and force `EI_max` if this
/// period's YTD EI reaches the annual EI maximum).
///
/// [`K2Method::YearToDate`] uses D/D1 + PR × period amounts.
#[allow(clippy::too_many_arguments)] // T4127 K2 inputs are inherently many; pack later if needed.
pub fn k2(
    lowest_rate: Rate,
    pay_periods: PayPeriod,
    period_cpp: Money,
    ytd_cpp_before_period: Money,
    period_ei: Money,
    ytd_ei_before_period: Money,
    periods_remaining: u16,
    cpp_months: u8,
    method: K2Method,
    cpp: &CppParams,
    ei: &EiParams,
) -> Result<Money, CreditsError> {
    if periods_remaining == 0 || periods_remaining > pay_periods.get() {
        return Err(CreditsError::InvalidPeriodsRemaining {
            got: periods_remaining,
        });
    }
    let p = integer_money(pay_periods.get())?;
    let pr = integer_money(periods_remaining)?;
    let one = Money::parse("1").map_err(decimal)?;
    let prorated_base_max = times_pm_over_twelve(cpp.base_max, cpp_months).map_err(decimal)?;
    let base_total =
        Ratio::div(&cpp.base_rate.to_string(), &cpp.total_rate.to_string()).map_err(decimal)?;

    let cpp_base = match method {
        K2Method::PdocObserved => {
            let period_annualized =
                k2_cpp_annualized_base_before_max_rule(pay_periods, period_cpp, cpp_months, cpp)?;
            let ytd_base = ytd_cpp_before_period
                .checked_mul_ratio(base_total)
                .map_err(decimal)?;
            let candidate = if period_annualized > ytd_base {
                period_annualized
            } else {
                ytd_base
            };
            cap_cpp_base(candidate, prorated_base_max, one)?
        }
        K2Method::T4127Literal => {
            let total_after = ytd_cpp_before_period
                .checked_add(period_cpp)
                .map_err(decimal)?;
            if total_after >= cpp.total_max {
                prorated_base_max
            } else {
                let raw = k2_cpp_annualized_base_before_max_rule(
                    pay_periods,
                    period_cpp,
                    cpp_months,
                    cpp,
                )?;
                cap_cpp_base(raw, prorated_base_max, one)?
            }
        }
        K2Method::YearToDate => {
            let projected_total = ytd_cpp_before_period
                .checked_add(period_cpp.checked_mul(pr).map_err(decimal)?)
                .map_err(decimal)?
                .min(cpp.total_max);
            let projected_base = projected_total
                .checked_mul_ratio(base_total)
                .map_err(decimal)?;
            cap_cpp_base(projected_base, prorated_base_max, one)?
        }
    };

    let ei_annualized = period_ei.checked_mul(p).map_err(decimal)?;
    let ei_base = match method {
        K2Method::PdocObserved => ei_annualized,
        K2Method::T4127Literal => {
            let total_after = ytd_ei_before_period
                .checked_add(period_ei)
                .map_err(decimal)?;
            if total_after >= ei.employee_max {
                ei.employee_max
            } else {
                ei_annualized
            }
        }
        K2Method::YearToDate => ytd_ei_before_period
            .checked_add(period_ei.checked_mul(pr).map_err(decimal)?)
            .map_err(decimal)?,
    }
    .min(ei.employee_max);

    let credit_base = Money::from_decimal(cpp_base.as_decimal())
        .checked_add(ei_base)
        .map_err(decimal)?;
    rounded_rate_product(credit_base, lowest_rate)
}

fn cap_cpp_base(
    candidate: Money,
    prorated_base_max: Ratio,
    one: Money,
) -> Result<Ratio, CreditsError> {
    if candidate.cmp_ratio(&prorated_base_max) == Ordering::Greater {
        Ok(prorated_base_max)
    } else {
        candidate.checked_div(one).map_err(decimal)
    }
}

/// Raw period-annualized CPP-credit base `(P × C × base/total)` before the
/// YTD comparison or the reaching-period maximum rule.
///
/// Test 73 uses this to prove period 20's raw figure is below both `D×ratio`
/// and `base_max`.
pub fn k2_cpp_annualized_base_before_max_rule(
    pay_periods: PayPeriod,
    period_cpp: Money,
    _cpp_months: u8,
    cpp: &CppParams,
) -> Result<Money, CreditsError> {
    let ratio =
        Ratio::div(&cpp.base_rate.to_string(), &cpp.total_rate.to_string()).map_err(decimal)?;
    period_cpp
        .checked_mul(integer_money(pay_periods.get())?)
        .and_then(|v| v.checked_mul_ratio(ratio))
        .map_err(decimal)
}
