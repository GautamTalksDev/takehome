//! British Columbia tax reduction S (T4127 Chapter 4).
//!
//! Unlike Ontario, BC S is a three-band schedule on A:
//! - A ≤ $25,570 → min(T4, S2)
//! - $25,570 < A ≤ upper → min(T4, S2 − (A − $25,570) × 3.56%)
//! - A > upper → 0
//!
//! S2 and the upper threshold are OptionScoped on `TaxReduction` (basic / dependant).
//! January 2026: S2 = $575, upper = $41,722.
//! July 2026 option 1: S2 = $805, upper = $44,952.
//! July 2026 option 2: S2 = $690, upper = $44,952.
//! The $25,570 start and 3.56% phase-out rate are the same in both editions.

use crate::decimal::{DecimalError, Money, Rate};
use crate::rounding::round_tax_to_cent;
use crate::rules::schema::TaxReduction;

/// BC provincial tax reduction S.
pub fn british_columbia_s(
    a: Money,
    t4: Money,
    reduction: &TaxReduction,
) -> Result<Money, DecimalError> {
    let start = Money::parse("25570.00")?;
    let upper = reduction.dependant;
    let phaseout = Rate::parse("0.0356")?;
    if a > upper {
        return Ok(Money::ZERO);
    }
    let available = if a <= start {
        reduction.basic
    } else {
        reduction
            .basic
            .checked_sub(a.checked_sub(start)?.checked_mul_rate(phaseout)?)?
            .floor_at_zero()
    };
    Ok(round_tax_to_cent(
        t4.min(available).checked_div(Money::parse("1")?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::british_columbia_s;
    use crate::decimal::Money;
    use crate::rules::schema::TaxReduction;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn reduction(basic: &str, upper: &str) -> TaxReduction {
        TaxReduction {
            basic: money(basic),
            dependant: money(upper),
        }
    }

    /// Test 27 — January: base 575, thresholds 25570 / 41722, rate 0.0356.
    #[test]
    fn january_s_schedule_and_cap_at_t4() {
        let red = reduction("575.00", "41722.00");
        let t4 = money("2000.00");
        assert_eq!(
            british_columbia_s(money("25570.00"), t4, &red).unwrap(),
            money("575.00")
        );
        // A = 30,000 is inside 25570–41722: 575 − (30000 − 25570) × 0.0356 = 417.29
        assert_eq!(
            british_columbia_s(money("30000.00"), t4, &red).unwrap(),
            money("417.29")
        );
        assert_eq!(
            british_columbia_s(money("41722.01"), t4, &red).unwrap(),
            money("0.00")
        );
        assert_eq!(
            british_columbia_s(money("10000.00"), money("100.00"), &red).unwrap(),
            money("100.00")
        );
    }

    /// Test 27 — July option 1: base 805, upper 44952, capped at T4.
    #[test]
    fn july_option1_s_schedule_and_cap_at_t4() {
        let red = reduction("805.00", "44952.00");
        let t4 = money("2000.00");
        assert_eq!(
            british_columbia_s(money("25570.00"), t4, &red).unwrap(),
            money("805.00")
        );
        assert_eq!(
            british_columbia_s(money("44952.01"), t4, &red).unwrap(),
            money("0.00")
        );
        assert_eq!(
            british_columbia_s(money("10000.00"), money("100.00"), &red).unwrap(),
            money("100.00")
        );
    }

    /// Test 27 — July option 2: base 690, upper 44952, capped at T4.
    #[test]
    fn july_option2_s_schedule_and_cap_at_t4() {
        let red = reduction("690.00", "44952.00");
        let t4 = money("2000.00");
        assert_eq!(
            british_columbia_s(money("25570.00"), t4, &red).unwrap(),
            money("690.00")
        );
        assert_eq!(
            british_columbia_s(money("44952.01"), t4, &red).unwrap(),
            money("0.00")
        );
        assert_eq!(
            british_columbia_s(money("10000.00"), money("50.00"), &red).unwrap(),
            money("50.00")
        );
    }
}
