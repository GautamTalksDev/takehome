//! Alberta supplemental credit K5P (T4127 Chapter 4).
//!
//! `K5P = max(0, ((K1P + K2P) − 4896.00) × 0.25)`.
//! The threshold and rate are Chapter 4 constants; schema `SupplementalCredit`
//! is a presence flag only.

use crate::decimal::{DecimalError, Money, Rate};
use crate::rounding::round_tax_to_cent;

/// Amount of K1P+K2P below which K5P is zero.
pub const K5P_THRESHOLD: &str = "4896.00";
/// K5P rate on the excess above [`K5P_THRESHOLD`].
pub const K5P_RATE: &str = "0.25";

/// Alberta K5P, floored at zero.
pub fn alberta_k5p(k1p: Money, k2p: Money) -> Result<Money, DecimalError> {
    let threshold = Money::parse(K5P_THRESHOLD)?;
    let rate = Rate::parse(K5P_RATE)?;
    let excess = k1p
        .checked_add(k2p)?
        .checked_sub(threshold)?
        .floor_at_zero();
    Ok(round_tax_to_cent(
        excess
            .checked_mul_rate(rate)?
            .checked_div(Money::parse("1")?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::alberta_k5p;
    use crate::decimal::Money;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    /// Test 26 — floor case: credits at or below the threshold yield 0.
    #[test]
    fn k5p_floors_at_zero() {
        assert_eq!(
            alberta_k5p(money("4896.00"), money("0.00")).unwrap(),
            money("0.00")
        );
        assert_eq!(
            alberta_k5p(money("2000.00"), money("2000.00")).unwrap(),
            money("0.00")
        );
    }

    /// Test 26 — just above the floor: 0.04 × 0.25 = 0.01.
    #[test]
    fn k5p_just_above_the_floor() {
        assert_eq!(
            alberta_k5p(money("4896.04"), money("0.00")).unwrap(),
            money("0.01")
        );
    }
}
