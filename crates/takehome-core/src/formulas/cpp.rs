//! Canada Pension Plan period contributions (C and C2).
//!
//! Spec §6.11: intermediates are never rounded. Every `max × PM/12` in this
//! engine is a [`Ratio`]; only the final C / C2 passes through
//! [`round_contribution_to_cent`].

use std::cmp::Ordering;

use crate::decimal::{DecimalError, Money, Ratio};
use crate::request::PayPeriod;
use crate::rounding::{round_contribution_to_cent, truncate_exemption_to_cent};
use crate::rules::schema::CppParams;

fn integer_money(value: u16) -> Result<Money, DecimalError> {
    Money::parse(&value.to_string())
}

fn one() -> Result<Money, DecimalError> {
    Money::parse("1")
}

fn twelve() -> Result<Money, DecimalError> {
    Money::parse("12")
}

/// `amount × PM / 12` as an unrounded [`Ratio`].
///
/// Spec §6.11 — this intermediate is never rounded and never materialised as
/// [`Money`]. CPP `total_max`, CPP2 `second_additional_max`, K2 `base_max`,
/// and the YMPE / YAMPE thresholds all go through here.
pub(crate) fn times_pm_over_twelve(amount: Money, months: u8) -> Result<Ratio, DecimalError> {
    amount
        .checked_mul(Money::parse(&months.to_string())?)?
        .checked_div(twelve()?)
}

/// Remaining room under `amount × PM/12` after year-to-date, as a [`Ratio`].
///
/// `(amount × PM − ytd × 12) / 12` — algebraically `prorated_max − ytd`, kept
/// off the [`Money`] type so the 0.005 in `2115.225 − 2115.22` survives until
/// [`round_contribution_to_cent`] on the final lesser-of.
fn remaining_room(amount: Money, months: u8, ytd: Money) -> Result<Ratio, DecimalError> {
    let pm = Money::parse(&months.to_string())?;
    let t = twelve()?;
    amount
        .checked_mul(pm)?
        .checked_sub(ytd.checked_mul(t)?)?
        .checked_div(t)
}

fn min_ratio(a: Ratio, b: Ratio) -> Ratio {
    if a.as_decimal() <= b.as_decimal() {
        a
    } else {
        b
    }
}

fn floor_ratio_at_zero(value: Ratio) -> Result<Ratio, DecimalError> {
    if value.as_decimal().is_sign_negative() && !value.as_decimal().is_zero() {
        Money::ZERO.checked_div(one()?)
    } else {
        Ok(value)
    }
}

fn money_as_ratio(amount: Money) -> Result<Ratio, DecimalError> {
    amount.checked_div(one()?)
}

fn ratio_sub(lhs: Ratio, rhs: Ratio) -> Result<Ratio, DecimalError> {
    money_as_ratio(Money::from_decimal(lhs.as_decimal() - rhs.as_decimal()))
}

/// CPP C for one period, capped by the employee's remaining annual maximum.
///
/// `C = round_contribution_to_cent( lesser of [total_max × PM/12 − D]
/// and [rate × (PI − truncate_exemption(3500 × PM/(12P)))] )`, floored at 0.
pub fn cpp_contribution(
    pensionable_earnings: Money,
    ytd_cpp: Money,
    pay_periods: PayPeriod,
    cpp_months: u8,
    params: &CppParams,
    exempt: bool,
) -> Result<Money, DecimalError> {
    let annual_max = times_pm_over_twelve(params.total_max, cpp_months)?;
    if exempt
        || pensionable_earnings.is_negative()
        || pensionable_earnings.is_zero()
        || ytd_cpp.cmp_ratio(&annual_max) != Ordering::Less
    {
        return Ok(Money::ZERO);
    }

    let p = integer_money(pay_periods.get())?;
    let months = Money::parse(&cpp_months.to_string())?;
    let period_exemption = truncate_exemption_to_cent(
        params
            .basic_exemption
            .checked_mul(months)?
            .checked_div(twelve()?.checked_mul(p)?)?,
    );
    let contributory = pensionable_earnings
        .checked_sub(period_exemption)?
        .floor_at_zero();
    let uncapped = contributory
        .checked_mul_rate(params.total_rate)?
        .checked_div(one()?)?;
    let remaining = floor_ratio_at_zero(remaining_room(params.total_max, cpp_months, ytd_cpp)?)?;
    Ok(round_contribution_to_cent(min_ratio(uncapped, remaining)))
}

/// Second additional CPP contribution C2 for one period.
///
/// T4127 (salary/wages):
/// ```text
/// W  = greater of PI_YTD and (YMPE × PM/12)
/// C2 = lesser of [second_additional_max × PM/12 − D2]
///              and [(PI_YTD + PI − W) × second_additional_rate]
/// ```
/// Negative (ii) floors to zero. C2 is strictly year-to-date: it does not
/// begin until accumulated pensionable earnings exceed YMPE, regardless of
/// the annualized rate. Spec §21.5 falls out of `W`.
pub fn cpp2_contribution(
    pensionable_earnings: Money,
    ytd_pensionable_earnings: Money,
    ytd_cpp2: Money,
    cpp_months: u8,
    params: &CppParams,
    exempt: bool,
) -> Result<Money, DecimalError> {
    let annual_max = times_pm_over_twelve(params.second_additional_max, cpp_months)?;
    if exempt
        || pensionable_earnings.is_negative()
        || pensionable_earnings.is_zero()
        || ytd_cpp2.cmp_ratio(&annual_max) != Ordering::Less
    {
        return Ok(Money::ZERO);
    }

    // W = greater of PI_YTD and (YMPE × PM/12). Kept as Ratio.
    let ympe_prorated = times_pm_over_twelve(params.ympe, cpp_months)?;
    let w = if ytd_pensionable_earnings.cmp_ratio(&ympe_prorated) == Ordering::Greater {
        money_as_ratio(ytd_pensionable_earnings)?
    } else {
        ympe_prorated
    };

    // (PI_YTD + PI − W) × rate, floored at 0.
    let accumulated = ytd_pensionable_earnings.checked_add(pensionable_earnings)?;
    let excess = floor_ratio_at_zero(ratio_sub(money_as_ratio(accumulated)?, w)?)?;
    let uncapped = Money::from_decimal(excess.as_decimal())
        .checked_mul_rate(params.second_additional_rate)?
        .checked_div(one()?)?;

    let remaining = floor_ratio_at_zero(remaining_room(
        params.second_additional_max,
        cpp_months,
        ytd_cpp2,
    )?)?;
    Ok(round_contribution_to_cent(min_ratio(uncapped, remaining)))
}

#[cfg(test)]
mod tests {
    use super::{
        cpp2_contribution, cpp_contribution, money_as_ratio, remaining_room, times_pm_over_twelve,
    };
    use crate::decimal::{Money, Rate, Ratio};
    use crate::request::PayPeriod;
    use crate::rounding::{round_contribution_to_cent, truncate_exemption_to_cent};
    use crate::rules::schema::CppParams;
    use proptest::prelude::*;
    use std::cmp::Ordering;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    fn canada_cpp() -> CppParams {
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

    fn quebec_qpp() -> CppParams {
        let mut params = canada_cpp();
        params.total_rate = rate("0.0630");
        params.total_max = money("4479.30");
        params
    }

    fn weekly() -> PayPeriod {
        PayPeriod::new(52).unwrap()
    }
    fn monthly() -> PayPeriod {
        PayPeriod::new(12).unwrap()
    }

    /// Simulate the class of bug 41b exists to make unreachable: materialise
    /// the prorated cap as [`Money`], then take `min(rounded_uncapped, cap − D)`.
    fn c_with_money_cap(
        cap: Money,
        pensionable_earnings: Money,
        ytd_cpp: Money,
        pay_periods: PayPeriod,
        cpp_months: u8,
        params: &CppParams,
    ) -> Money {
        if ytd_cpp >= cap {
            return Money::ZERO;
        }
        let remaining = cap.checked_sub(ytd_cpp).unwrap().floor_at_zero();
        let p = money(&pay_periods.get().to_string());
        let months = money(&cpp_months.to_string());
        let period_exemption = truncate_exemption_to_cent(
            params
                .basic_exemption
                .checked_mul(months)
                .unwrap()
                .checked_div(money("12").checked_mul(p).unwrap())
                .unwrap(),
        );
        let contributory = pensionable_earnings
            .checked_sub(period_exemption)
            .unwrap()
            .floor_at_zero();
        let uncapped = round_contribution_to_cent(
            contributory
                .checked_mul_rate(params.total_rate)
                .unwrap()
                .checked_div(money("1"))
                .unwrap(),
        );
        uncapped.min(remaining)
    }

    /// 36. Weekly P=52, PI=1000.00, D=0, PM=12.
    ///
    /// Exemption 67.30 (truncated). 0.0595 × 932.70 = 55.49565 → half-up → 55.50.
    /// T4127 §6.13 prints C = $55.50 for weekly $1,000. Not 55.49 (truncation
    /// leaking from the exemption rule).
    #[test]
    fn weekly_1000_contribution_is_55_50() {
        let got = cpp_contribution(
            money("1000.00"),
            money("0.00"),
            weekly(),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(got, money("55.50"));
        let exemption =
            truncate_exemption_to_cent(money("3500.00").checked_div(money("52")).unwrap());
        assert_eq!(exemption, money("67.30"));
        let product = (money("1000.00").checked_sub(exemption).unwrap()) * rate("0.0595");
        assert_eq!(
            round_contribution_to_cent(product.checked_div(money("1")).unwrap()),
            money("55.50")
        );
    }

    /// 37a. Engine-boundary re-assert of M0 tests 1/2: the exemption canary.
    /// Fails loudly if someone swaps truncate vs contribution-round at the call site.
    #[test]
    fn exemption_canary_truncate_not_contribution_round() {
        let monthly_exemption = money("3500.00").checked_div(money("12")).unwrap();
        assert_eq!(
            truncate_exemption_to_cent(monthly_exemption).to_string(),
            "291.66"
        );
        assert_eq!(
            round_contribution_to_cent(monthly_exemption).to_string(),
            "291.67"
        );
    }

    /// 37b. PI = 5000.15, P = 12: truncated exemption yields C = 280.16;
    /// the rounded exemption would yield 280.15. At PI = 5000.00 the two
    /// exemptions collapse to the same cent — finding a PI that splits them
    /// is the test.
    #[test]
    fn monthly_5000_15_uses_truncated_exemption_280_16() {
        let params = canada_cpp();
        let pi = money("5000.15");
        let got = cpp_contribution(pi, money("0.00"), monthly(), 12, &params, false).unwrap();
        assert_eq!(got, money("280.16"));

        let input = money("3500.00").checked_div(money("12")).unwrap();
        let truncated = truncate_exemption_to_cent(input);
        let rounded = round_contribution_to_cent(input);
        assert_eq!(truncated, money("291.66"));
        assert_eq!(rounded, money("291.67"));
        let c_trunc = round_contribution_to_cent(
            (pi.checked_sub(truncated).unwrap() * params.total_rate)
                .checked_div(money("1"))
                .unwrap(),
        );
        let c_round = round_contribution_to_cent(
            (pi.checked_sub(rounded).unwrap() * params.total_rate)
                .checked_div(money("1"))
                .unwrap(),
        );
        assert_eq!(c_trunc, money("280.16"));
        assert_eq!(c_round, money("280.15"));
        assert_ne!(
            c_trunc, c_round,
            "PI=5000.15 is the split; PI=5000.00 would collapse both paths to 280.15"
        );
        assert_eq!(got, c_trunc);
    }

    /// 38. At the cap: D = 4200.00, large PI → C = 30.45, not the rate figure.
    #[test]
    fn near_annual_max_uses_remainder() {
        let got = cpp_contribution(
            money("40000.00"),
            money("4200.00"),
            weekly(),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(got, money("30.45"));
        assert_ne!(got, money("55.50"));
    }

    /// 39. Past the cap: D = 4230.45 → C = 0.00, never negative.
    #[test]
    fn at_annual_max_is_zero_never_negative() {
        let got = cpp_contribution(
            money("1000.00"),
            money("4230.45"),
            weekly(),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(got, money("0.00"));
        assert!(!got.is_negative());
    }

    /// 40. exempt=true → C = 0.00 and C2 = 0.00.
    #[test]
    fn exempt_zeros_c_and_c2() {
        let params = canada_cpp();
        let c =
            cpp_contribution(money("1000.00"), money("0.00"), weekly(), 12, &params, true).unwrap();
        let c2 = cpp2_contribution(
            money("2000.00"),
            money("0.00"),
            money("0.00"),
            12,
            &params,
            true,
        )
        .unwrap();
        assert_eq!(c, money("0.00"));
        assert_eq!(c2, money("0.00"));
    }

    /// 41. PM = 6, D = 0, PI large enough that the cap binds: assert C, not the
    ///     cap. 4230.45 × 6/12 is a Ratio (2115.225); C is that remaining after
    ///     round_contribution_to_cent.
    #[test]
    fn pm6_cap_binds_asserts_c_not_the_cap() {
        let params = canada_cpp();
        let cap: Ratio = times_pm_over_twelve(params.total_max, 6).unwrap();
        assert_eq!(cap, Ratio::parse("2115.225").unwrap());
        let c: Money = cpp_contribution(
            money("40000.00"),
            money("0.00"),
            weekly(),
            6,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(c, money("2115.23"));
        assert_ne!(
            c.to_string(),
            cap.to_string(),
            "C is Money after round_contribution_to_cent; the cap stays a Ratio"
        );
    }

    /// 41b. The prorated cap is never materialised as a Money.
    ///
    /// Signature: `times_pm_over_twelve` returns Ratio; `cpp_contribution`
    /// returns Money only for C. Cap-binding case: ytd_cpp just under the
    /// unrounded prorated maximum so the remaining-room branch binds.
    ///
    /// 4230.45 × 6/12 = 2115.225. Materialising that as 2115.22 or as 2115.23
    /// both produce a remaining that is not 0.005; rounding either remaining
    /// as if it were the cap is a different computation than rounding the
    /// Ratio remaining. The 2115.22 Money-cap path also yields a different
    /// final C (0.00 vs 0.01).
    #[test]
    fn prorated_cap_is_never_money_and_rounding_it_changes_c() {
        let params = canada_cpp();
        let cap: Ratio = times_pm_over_twelve(params.total_max, 6).unwrap();
        let c_type_probe: Money = cpp_contribution(
            money("10000.00"),
            money("2115.22"),
            weekly(),
            6,
            &params,
            false,
        )
        .unwrap();
        let _ = (cap, c_type_probe);

        assert_eq!(cap, Ratio::parse("2115.225").unwrap());
        assert_eq!(
            money("2115.22").cmp_ratio(&cap),
            Ordering::Less,
            "truncated Money cap sits strictly below the Ratio"
        );
        assert_eq!(
            money("2115.23").cmp_ratio(&cap),
            Ordering::Greater,
            "half-up Money cap sits strictly above the Ratio"
        );

        let ytd = money("2115.22");
        let pi = money("10000.00");
        let remaining = remaining_room(params.total_max, 6, ytd).unwrap();
        assert_eq!(remaining, Ratio::parse("0.005").unwrap());
        let remaining_22 = money_as_ratio(money("2115.22").checked_sub(ytd).unwrap()).unwrap();
        let remaining_23 = money_as_ratio(money("2115.23").checked_sub(ytd).unwrap()).unwrap();
        assert_ne!(
            remaining, remaining_22,
            "Money 2115.22 remaining is 0, not 0.005"
        );
        assert_ne!(
            remaining, remaining_23,
            "Money 2115.23 remaining is 0.01, not 0.005"
        );

        let c = cpp_contribution(pi, ytd, weekly(), 6, &params, false).unwrap();
        assert_eq!(c, round_contribution_to_cent(remaining));
        assert_eq!(c, money("0.01"));

        let c_22 = c_with_money_cap(money("2115.22"), pi, ytd, weekly(), 6, &params);
        let c_23 = c_with_money_cap(money("2115.23"), pi, ytd, weekly(), 6, &params);
        assert_eq!(c_22, money("0.00"));
        assert_eq!(c_23, money("0.01"));
        assert_ne!(
            c_22, c,
            "materialising the cap as 2115.22 treats ytd as already at the max"
        );
        // 2115.23 − 2115.22 = 0.01, which equals round(0.005) for this D, so
        // the half-up Money cap can sneak through a C assertion. The Ratio
        // remaining (0.005 ≠ 0.01) is what makes that class of bug visible.
        assert_ne!(remaining.to_string(), c_23.to_string());
        assert_ne!(c_22, c_23);
    }

    /// CPP2: 416.00 × PM/12 is a Ratio. Cap-binding remainder is rounded only
    /// as the final C2. Needs PI_YTD above YMPE so the earnings term is positive.
    #[test]
    fn cpp2_prorated_max_is_ratio_not_money() {
        let params = canada_cpp();
        let cap: Ratio = times_pm_over_twelve(params.second_additional_max, 1).unwrap();
        assert_eq!(money("34.66").cmp_ratio(&cap), Ordering::Less);
        assert_eq!(money("34.67").cmp_ratio(&cap), Ordering::Greater);
        assert!(
            cap.to_string().contains("34.66") && cap.to_string() != "34.66",
            "416 × 1/12 must retain sub-cent scale, got {cap}"
        );

        let remaining = remaining_room(params.second_additional_max, 1, money("34.66")).unwrap();
        assert_ne!(remaining.to_string(), "0.00");
        assert_ne!(remaining.to_string(), "0.01");
        let c2: Money = cpp2_contribution(
            money("2000.00"),
            money("80000.00"),
            money("34.66"),
            1,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(c2, round_contribution_to_cent(remaining));
    }

    /// D-003 regression: weekly $2000, zero YTD → C2 = 0.00.
    /// The annualized-band reading produced 8.00; the literal W form floors.
    #[test]
    fn cpp2_zero_ytd_high_salary_is_zero() {
        let c2 = cpp2_contribution(
            money("2000.00"),
            money("0.00"),
            money("0.00"),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(c2, money("0.00"));
    }

    /// YTD just below YMPE: only the dollars that cross YMPE this period.
    /// PI_YTD=74500, PI=2000 → excess = 74500+2000−74600 = 1900 → ×0.04 = 76.00.
    #[test]
    fn cpp2_ytd_just_below_ympe_is_partial() {
        let c2 = cpp2_contribution(
            money("2000.00"),
            money("74500.00"),
            money("0.00"),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(c2, money("76.00"));
    }

    /// YTD above YMPE: full period band (capped by remaining room).
    /// PI_YTD=80000, PI=2000 → (80000+2000−80000)×0.04 = 80.00.
    #[test]
    fn cpp2_ytd_above_ympe_charges_period_band() {
        let c2 = cpp2_contribution(
            money("2000.00"),
            money("80000.00"),
            money("0.00"),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(c2, money("80.00"));
    }

    /// Cap binds when remaining room under 416 − D2 is smaller than the band.
    #[test]
    fn cpp2_remaining_room_cap_binds() {
        let c2 = cpp2_contribution(
            money("2000.00"),
            money("84000.00"),
            money("400.00"),
            12,
            &canada_cpp(),
            false,
        )
        .unwrap();
        assert_eq!(c2, money("16.00"));
    }

    /// Spec §21.5: C2 stays 0.00 in the period CPP caps, when YTD PE has not
    /// yet crossed YMPE. C = remaining room; (PI_YTD+PI−W) is still negative.
    #[test]
    fn cpp2_stays_zero_in_period_cpp_caps_below_ympe() {
        let params = canada_cpp();
        let c = cpp_contribution(
            money("500.00"),
            money("4220.00"),
            weekly(),
            12,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(c, money("10.45"), "C takes the remaining annual room");
        let c2 = cpp2_contribution(
            money("500.00"),
            money("74000.00"),
            money("0.00"),
            12,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(c2, money("0.00"));
    }

    /// 46. Quebec params: same shape, different constants, no hard-coded rate.
    #[test]
    fn quebec_params_same_shape_different_constants() {
        let qpp = quebec_qpp();
        let cpp = canada_cpp();
        assert_eq!(qpp.total_rate, rate("0.0630"));
        assert_eq!(qpp.total_max, money("4479.30"));
        let q =
            cpp_contribution(money("1000.00"), money("0.00"), weekly(), 12, &qpp, false).unwrap();
        let c =
            cpp_contribution(money("1000.00"), money("0.00"), weekly(), 12, &cpp, false).unwrap();
        assert_eq!(q, money("58.76")); // 0.0630 × 932.70
        assert_eq!(c, money("55.50"));
        assert_ne!(q, c);
    }

    // 47. Property: C <= round_contribution(total_max × PM/12).
    proptest! {
        #[test]
        fn c_never_exceeds_rounded_prorated_max(
            pi_cents in 0u32..=10_000_000,
            d_cents in 0u32..=500_000,
            pm in 1u8..=12,
            exempt in proptest::bool::ANY,
        ) {
            let pi = money(&format!("{}.{:02}", pi_cents / 100, pi_cents % 100));
            let d = money(&format!("{}.{:02}", d_cents / 100, d_cents % 100));
            let params = canada_cpp();
            let got = cpp_contribution(pi, d, weekly(), pm, &params, exempt).unwrap();
            let max_c = round_contribution_to_cent(times_pm_over_twelve(params.total_max, pm).unwrap());
            prop_assert!(got <= max_c);
        }
    }

    // 48. Property: C >= 0 always.
    proptest! {
        #[test]
        fn c_never_negative(
            pi_cents in 0u32..=10_000_000,
            d_cents in 0u32..=500_000,
            pm in 1u8..=12,
            exempt in proptest::bool::ANY,
        ) {
            let pi = money(&format!("{}.{:02}", pi_cents / 100, pi_cents % 100));
            let d = money(&format!("{}.{:02}", d_cents / 100, d_cents % 100));
            let got = cpp_contribution(pi, d, weekly(), pm, &canada_cpp(), exempt).unwrap();
            prop_assert!(!got.is_negative());
        }
    }

    // 49. Property: C is monotonically non-decreasing in PI.
    proptest! {
        #[test]
        fn c_monotone_in_pi(
            a_cents in 0u32..=10_000_000,
            b_cents in 0u32..=10_000_000,
            d_cents in 0u32..=200_000,
        ) {
            prop_assume!(a_cents <= b_cents);
            let params = canada_cpp();
            let d = money(&format!("{}.{:02}", d_cents / 100, d_cents % 100));
            let a = money(&format!("{}.{:02}", a_cents / 100, a_cents % 100));
            let b = money(&format!("{}.{:02}", b_cents / 100, b_cents % 100));
            let ca = cpp_contribution(a, d, weekly(), 12, &params, false).unwrap();
            let cb = cpp_contribution(b, d, weekly(), 12, &params, false).unwrap();
            prop_assert!(ca <= cb);
        }
    }
}
