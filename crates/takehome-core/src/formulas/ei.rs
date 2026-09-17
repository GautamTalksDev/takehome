//! Employment insurance and QPIP premiums (T4127 Chapter 6 / Table 8.7–8.8).
//!
//! Tests only (Step 6). Implementation follows once the suite compiles red
//! for missing `ei_premium` / `employer_ei_premium` / `qpip_premium`.

use crate::decimal::{DecimalError, Money, Rate};
use crate::request::Province;
use crate::rounding::round_contribution_to_cent;
use crate::rules::schema::{EiParams, QpipParams};

fn premium(
    earnings: Money,
    ytd: Money,
    rate: Rate,
    annual_max: Money,
    exempt: bool,
) -> Result<Money, DecimalError> {
    if exempt || earnings.is_negative() || earnings.is_zero() || ytd >= annual_max {
        return Ok(Money::ZERO);
    }
    let remaining = annual_max.checked_sub(ytd)?.floor_at_zero();
    let raw = earnings.checked_mul_rate(rate)?;
    let rounded = round_contribution_to_cent(raw.checked_div(Money::parse("1")?)?);
    Ok(rounded.min(remaining).floor_at_zero())
}

/// Employee EI premium for the period, capped by the annual maximum.
pub fn ei_premium(
    insurable_earnings: Money,
    ytd_ei: Money,
    params: &EiParams,
    exempt: bool,
) -> Result<Money, DecimalError> {
    premium(
        insurable_earnings,
        ytd_ei,
        params.employee_rate,
        params.employee_max,
        exempt,
    )
}

/// Employer EI premium, calculated from the employer rate (not employee EI × 1.4).
pub fn employer_ei_premium(
    insurable_earnings: Money,
    ytd_employer_ei: Money,
    params: &EiParams,
    exempt: bool,
) -> Result<Money, DecimalError> {
    premium(
        insurable_earnings,
        ytd_employer_ei,
        params.employer_rate,
        params.employer_max,
        exempt,
    )
}

/// Employee QPIP premium. QPIP applies only in Quebec.
pub fn qpip_premium(
    insurable_earnings: Money,
    ytd_qpip: Money,
    province: Province,
    params: &QpipParams,
    exempt: bool,
) -> Result<Money, DecimalError> {
    if province != Province::Qc {
        return Ok(Money::ZERO);
    }
    premium(
        insurable_earnings,
        ytd_qpip,
        params.employee_rate,
        params.employee_max,
        exempt,
    )
}

#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Rate};
    use crate::request::Province;
    use crate::rounding::round_contribution_to_cent;
    use crate::rules::schema::{EiParams, QpipParams};
    use proptest::prelude::*;

    use super::{ei_premium, employer_ei_premium, qpip_premium};

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    fn canada_ei() -> EiParams {
        EiParams {
            max_insurable: money("68900.00"),
            employee_rate: rate("0.0163"),
            employee_max: money("1123.07"),
            employer_rate: rate("0.02282"),
            employer_max: money("1572.30"),
        }
    }

    fn quebec_ei() -> EiParams {
        // T4127 Table 8.7 — QC row.
        EiParams {
            max_insurable: money("68900.00"),
            employee_rate: rate("0.0130"),
            employee_max: money("895.70"),
            employer_rate: rate("0.01820"),
            employer_max: money("1253.98"),
        }
    }

    fn qpip_params() -> QpipParams {
        QpipParams {
            max_insurable: money("103000.00"),
            employee_rate: rate("0.00430"),
            employee_max: money("442.90"),
            employer_rate: rate("0.00602"),
            employer_max: money("620.06"),
        }
    }

    /// 50. IE = 1000.00 → EI = 16.30
    #[test]
    fn ie_1000_is_16_30() {
        let got = ei_premium(money("1000.00"), money("0.00"), &canada_ei(), false).unwrap();
        assert_eq!(got, money("16.30"));
        // 0.0163 × 1000.00 = 16.30 exactly; contribution rounder is still the path.
        let raw = money("1000.00") * rate("0.0163");
        assert_eq!(
            round_contribution_to_cent(raw.checked_div(money("1")).unwrap()),
            money("16.30")
        );
    }

    /// 51. D1 = 1120.00 → EI = 3.07 (the remainder, not 16.30)
    #[test]
    fn near_cap_uses_remainder_not_rate_figure() {
        let got = ei_premium(money("1000.00"), money("1120.00"), &canada_ei(), false).unwrap();
        assert_eq!(got, money("3.07"));
        assert_ne!(got, money("16.30"));
    }

    /// 52. D1 = 1123.07 → EI = 0.00, never negative
    #[test]
    fn at_cap_ei_is_zero_never_negative() {
        let got = ei_premium(money("1000.00"), money("1123.07"), &canada_ei(), false).unwrap();
        assert_eq!(got, money("0.00"));
        assert!(!got.is_negative());
    }

    /// 53. Quebec rate 0.0130, max 895.70
    #[test]
    fn quebec_rate_and_max_from_params() {
        let params = quebec_ei();
        assert_eq!(params.employee_rate, rate("0.0130"));
        assert_eq!(params.employee_max, money("895.70"));
        let got = ei_premium(money("1000.00"), money("0.00"), &params, false).unwrap();
        assert_eq!(got, money("13.00")); // 0.0130 × 1000
        let at_cap = ei_premium(money("1000.00"), money("895.70"), &params, false).unwrap();
        assert_eq!(at_cap, money("0.00"));
    }

    /// 54. ei_exempt → 0.00
    #[test]
    fn ei_exempt_is_zero() {
        let got = ei_premium(money("1000.00"), money("0.00"), &canada_ei(), true).unwrap();
        assert_eq!(got, money("0.00"));
    }

    /// 55. QPIP: 0.00430 × IE, max 442.90, Quebec only, 0.00 elsewhere
    #[test]
    fn qpip_quebec_only() {
        let params = qpip_params();
        let qc = qpip_premium(
            money("1000.00"),
            money("0.00"),
            Province::Qc,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(qc, money("4.30"));

        let on = qpip_premium(
            money("1000.00"),
            money("0.00"),
            Province::On,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(on, money("0.00"));

        let capped = qpip_premium(
            money("1000.00"),
            money("442.90"),
            Province::Qc,
            &params,
            false,
        )
        .unwrap();
        assert_eq!(capped, money("0.00"));
    }

    // 56. Property: EI <= employee_max always
    proptest! {
        #[test]
        fn ei_never_exceeds_employee_max(
            ie_cents in 0u32..=10_000_000,
            d1_cents in 0u32..=200_000,
            exempt in proptest::bool::ANY,
        ) {
            let ie = money(&format!("{}.{:02}", ie_cents / 100, ie_cents % 100));
            let d1 = money(&format!("{}.{:02}", d1_cents / 100, d1_cents % 100));
            let params = canada_ei();
            let got = ei_premium(ie, d1, &params, exempt).unwrap();
            prop_assert!(got <= params.employee_max);
        }
    }

    // 57. Property: EI >= 0 always
    proptest! {
        #[test]
        fn ei_never_negative(
            ie_cents in 0u32..=10_000_000,
            d1_cents in 0u32..=200_000,
            exempt in proptest::bool::ANY,
        ) {
            let ie = money(&format!("{}.{:02}", ie_cents / 100, ie_cents % 100));
            let d1 = money(&format!("{}.{:02}", d1_cents / 100, d1_cents % 100));
            let got = ei_premium(ie, d1, &canada_ei(), exempt).unwrap();
            prop_assert!(!got.is_negative());
        }
    }

    /// 58. Employer EI = employer_rate × IE capped at employer_max — NOT 1.4 × employee.
    ///
    /// The 1.4× relationship is a property of the published rates (0.02282/0.0163),
    /// not a rule. Hard-coding it breaks the year one of them is set independently.
    #[test]
    fn employer_ei_is_rate_based_not_multiple_of_employee() {
        let params = canada_ei();
        let ie = money("1000.00");

        // Mid-range: both forms agree (1.4 × 16.30 = 22.82 = 0.02282 × 1000).
        let employee = ei_premium(ie, money("0.00"), &params, false).unwrap();
        assert_eq!(employee, money("16.30"));
        let employer = employer_ei_premium(ie, money("0.00"), &params, false).unwrap();
        assert_eq!(employer, money("22.82"));
        let one_point_four_times_employee =
            round_contribution_to_cent((employee * rate("1.4")).checked_div(money("1")).unwrap());
        assert_eq!(
            employer, one_point_four_times_employee,
            "mid-range: rate form and 1.4× employee must agree while rates stay in ratio"
        );

        // Cap case: employee remainder is 3.07 (D1=1120); 1.4×3.07=4.30, but
        // rate × IE is still 22.82. They must differ — proving we do not multiply.
        let employee_rem = ei_premium(ie, money("1120.00"), &params, false).unwrap();
        assert_eq!(employee_rem, money("3.07"));
        let employer_uncapped_ie = employer_ei_premium(ie, money("0.00"), &params, false).unwrap();
        let one_point_four_times_rem = round_contribution_to_cent(
            (employee_rem * rate("1.4"))
                .checked_div(money("1"))
                .unwrap(),
        );
        assert_eq!(one_point_four_times_rem, money("4.30"));
        assert_eq!(employer_uncapped_ie, money("22.82"));
        assert_ne!(
            employer_uncapped_ie, one_point_four_times_rem,
            "cap case: rate×IE must not collapse to 1.4× employee remainder"
        );
    }
}
