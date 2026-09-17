//! Basic personal amount formulas (BPAF / BPAMB / BPAYT) — T4127 §6.5 / Chapter 2.
//!
//! Tests only (Step 7). The phaseout ratio `(max − min) / (end − start)` must not
//! be pre-rounded before multiplying by `(NI − start)`.

use crate::decimal::{DecimalError, Money};
use crate::rounding::round_bpa_to_cent;
use crate::rules::schema::BasicPersonalAmount;
use thiserror::Error;

/// Basic-personal-amount calculation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BpaError {
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    #[error("basic personal amount is not applicable")]
    NotApplicable,
    #[error("same-as-federal BPA requires the federal definition")]
    MissingFederalReference,
}

/// Evaluate a fixed or income-phased basic personal amount.
pub fn basic_personal_amount(
    net_income: Money,
    definition: &BasicPersonalAmount,
) -> Result<Money, BpaError> {
    match definition {
        BasicPersonalAmount::Fixed { amount } => Ok(*amount),
        BasicPersonalAmount::Dynamic {
            max,
            min,
            phaseout_start,
            phaseout_end,
            ..
        } => {
            if net_income <= *phaseout_start {
                return Ok(*max);
            }
            if net_income >= *phaseout_end {
                return Ok(*min);
            }
            let range = phaseout_end.checked_sub(*phaseout_start)?;
            let reduction = max.checked_sub(*min)?;
            let elapsed = net_income.checked_sub(*phaseout_start)?;
            let ratio = reduction.checked_div(range)?;
            let amount = max.checked_sub(elapsed.checked_mul_ratio(ratio)?)?;
            Ok(round_bpa_to_cent(amount.checked_div(Money::parse("1")?)?))
        }
        BasicPersonalAmount::SameAsFederal => Err(BpaError::MissingFederalReference),
        BasicPersonalAmount::NotApplicable => Err(BpaError::NotApplicable),
    }
}

/// Resolve aliases before evaluating a basic personal amount.
pub fn resolve_basic_personal_amount(
    net_income: Money,
    definition: &BasicPersonalAmount,
    federal: Option<&BasicPersonalAmount>,
) -> Result<Money, BpaError> {
    match definition {
        BasicPersonalAmount::SameAsFederal => basic_personal_amount(
            net_income,
            federal.ok_or(BpaError::MissingFederalReference)?,
        ),
        _ => basic_personal_amount(net_income, definition),
    }
}

#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Rate, Ratio};
    use crate::rounding::round_bpa_to_cent;
    use crate::rules::schema::{BasicPersonalAmount, BpaFormula};
    use proptest::prelude::*;

    use super::{basic_personal_amount, resolve_basic_personal_amount};

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    /// Federal BPAF 2026 (T4127 Chapter 2).
    fn bpaf() -> BasicPersonalAmount {
        BasicPersonalAmount::Dynamic {
            formula: BpaFormula {},
            max: money("16452.00"),
            min: money("14829.00"),
            phaseout_start: money("181440.00"),
            phaseout_end: money("258482.00"),
        }
    }

    /// Manitoba BPAMB 2026 (T4127 Chapter 2) — same shape, different constants.
    fn bpamb() -> BasicPersonalAmount {
        BasicPersonalAmount::Dynamic {
            formula: BpaFormula {},
            max: money("15780.00"),
            min: money("0.00"),
            phaseout_start: money("200000.00"),
            phaseout_end: money("400000.00"),
        }
    }

    /// 59. NI = 100000 → 16452.00
    #[test]
    fn bpaf_below_phaseout_is_max() {
        assert_eq!(
            basic_personal_amount(money("100000.00"), &bpaf()).unwrap(),
            money("16452.00")
        );
    }

    /// 60. NI = 181440 → 16452.00 (boundary inclusive on the flat / max side)
    #[test]
    fn bpaf_at_phaseout_start_is_max() {
        assert_eq!(
            basic_personal_amount(money("181440.00"), &bpaf()).unwrap(),
            money("16452.00")
        );
    }

    /// 61. NI = 258482 → 14829.00
    #[test]
    fn bpaf_at_phaseout_end_is_min() {
        assert_eq!(
            basic_personal_amount(money("258482.00"), &bpaf()).unwrap(),
            money("14829.00")
        );
    }

    /// 62. NI = 300000 → 14829.00
    #[test]
    fn bpaf_above_phaseout_is_min() {
        assert_eq!(
            basic_personal_amount(money("300000.00"), &bpaf()).unwrap(),
            money("14829.00")
        );
    }

    /// 63. NI = 220000 → 16452 − (38560 × 1623/77042) = 15639.68
    ///
    /// Hand: 38560 × 1623 / 77042 = 812.32159…; 16452 − that = 15639.67840…;
    /// `round_bpa_to_cent` → 15639.68.
    /// Pre-rounding 1623/77042 to 4dp (0.0211) yields 15638.38 — a different cent.
    #[test]
    fn bpaf_mid_phaseout_unrounded_ratio_to_the_cent() {
        let got = basic_personal_amount(money("220000.00"), &bpaf()).unwrap();
        assert_eq!(got, money("15639.68"));

        // Prove the requirement: pre-rounding the ratio to 4dp changes the cent.
        let pre_rounded_ratio = Rate::parse("0.0211").unwrap(); // 1623/77042 half-up to 4dp
        let delta = money("38560.00"); // 220000 − 181440
        let pre_rounded_product = delta * pre_rounded_ratio;
        let pre_rounded_bpa = money("16452.00").checked_sub(pre_rounded_product).unwrap();
        let pre_rounded_cents = round_bpa_to_cent(
            pre_rounded_bpa
                .checked_div(money("1"))
                .expect("money as ratio"),
        );
        assert_eq!(pre_rounded_cents, money("15638.38"));
        assert_ne!(
            got, pre_rounded_cents,
            "pre-rounding 1623/77042 to 4dp must not be how BPAF is computed"
        );
    }

    /// 64. Result goes through rounding::round_bpa_to_cent (half-up at the cent).
    #[test]
    fn bpaf_uses_round_bpa_to_cent() {
        // Exact unrounded mid-phaseout value ends …7840…; half-up → …68, not truncate …67.
        let unrounded = Ratio::parse("15639.6784091794").unwrap();
        assert_eq!(round_bpa_to_cent(unrounded), money("15639.68"));
        let got = basic_personal_amount(money("220000.00"), &bpaf()).unwrap();
        assert_eq!(got, round_bpa_to_cent(unrounded));
    }

    /// 65. BPAMB: same shape, 15780 / 200000 / 400000, min 0.
    #[test]
    fn bpamb_same_shape_different_constants() {
        assert_eq!(
            basic_personal_amount(money("100000.00"), &bpamb()).unwrap(),
            money("15780.00")
        );
        assert_eq!(
            basic_personal_amount(money("200000.00"), &bpamb()).unwrap(),
            money("15780.00")
        );
        assert_eq!(
            basic_personal_amount(money("400000.00"), &bpamb()).unwrap(),
            money("0.00")
        );
        assert_eq!(
            basic_personal_amount(money("500000.00"), &bpamb()).unwrap(),
            money("0.00")
        );
        // Mid: 15780 − ((300000−200000) × (15780/200000)); exact then round_bpa.
        let mid = basic_personal_amount(money("300000.00"), &bpamb()).unwrap();
        assert_eq!(mid, money("7890.00"));
    }

    /// 66. BPAYT is identical to BPAF by reference (`SameAsFederal`), not duplicated constants.
    #[test]
    fn bpayt_resolves_by_reference_to_bpaf() {
        let fed = bpaf();
        let yt = BasicPersonalAmount::SameAsFederal;
        let ni = money("220000.00");
        let from_fed = basic_personal_amount(ni, &fed).unwrap();
        let from_yt = resolve_basic_personal_amount(ni, &yt, Some(&fed)).unwrap();
        assert_eq!(from_yt, from_fed);
        assert_eq!(from_yt, money("15639.68"));
        // Reference, not a second Dynamic copy with re-typed constants.
        assert!(matches!(yt, BasicPersonalAmount::SameAsFederal));
    }

    // 67. Property: BPAF is monotonically non-increasing in NI.
    proptest! {
        #[test]
        fn bpaf_monotone_non_increasing_in_ni(a_cents in 0u64..=50_000_000u64, b_cents in 0u64..=50_000_000u64) {
            prop_assume!(a_cents <= b_cents);
            let a = money(&format!("{}.{:02}", a_cents / 100, a_cents % 100));
            let b = money(&format!("{}.{:02}", b_cents / 100, b_cents % 100));
            let fa = basic_personal_amount(a, &bpaf()).unwrap();
            let fb = basic_personal_amount(b, &bpaf()).unwrap();
            prop_assert!(fa >= fb);
        }
    }

    // 68. Property: min <= BPAF <= max for all NI.
    proptest! {
        #[test]
        fn bpaf_within_min_max(ni_cents in 0u64..=100_000_000u64) {
            let ni = money(&format!("{}.{:02}", ni_cents / 100, ni_cents % 100));
            let got = basic_personal_amount(ni, &bpaf()).unwrap();
            prop_assert!(got >= money("14829.00"));
            prop_assert!(got <= money("16452.00"));
        }
    }
}
