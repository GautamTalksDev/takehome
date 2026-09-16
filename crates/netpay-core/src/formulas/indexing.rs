//! TD1 personal-amount indexing (T4127 Chapter 2).
//!
//! Indexed amount = `(1 + index_rate) × previous`, then
//! [`round_claim_to_dollar`](crate::rounding::round_claim_to_dollar).
//! Prince Edward Island does not index: `index_rate` is `None`, not zero.
//! Passing `None` is a hard error so a missing rate cannot silently no-op.

use crate::decimal::{DecimalError, Money, Rate};
use crate::rounding::round_claim_to_dollar;
use thiserror::Error;

/// Indexing failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IndexError {
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    #[error("jurisdiction does not index personal amounts (index_rate is absent, not zero)")]
    NotApplicable,
}

/// Apply a jurisdiction's published index rate to a previous-year personal amount.
///
/// `index_rate: None` (PE) errors. `Some(0)` would no-op; that is why PE must
/// not be stored as a zero rate.
pub fn index_claim_amount(previous: Money, index_rate: Option<Rate>) -> Result<Money, IndexError> {
    let rate = index_rate.ok_or(IndexError::NotApplicable)?;
    let increment = previous.checked_mul_rate(rate)?;
    let unrounded = previous.checked_add(increment)?;
    Ok(round_claim_to_dollar(
        unrounded.checked_div(Money::parse("1")?)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{index_claim_amount, IndexError};
    use crate::decimal::{Money, Rate};

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    #[test]
    fn absent_index_rate_is_an_error_not_a_noop() {
        let previous = money("15000.00");
        let err = index_claim_amount(previous, None).expect_err("None must error");
        assert_eq!(err, IndexError::NotApplicable);

        let zeroed = index_claim_amount(previous, Some(rate("0"))).expect("zero rate applies");
        assert_eq!(
            zeroed, previous,
            "a stored 0.0 index rate would silently no-op; PE must use None"
        );
    }

    #[test]
    fn federal_2026_index_2_percent_rounds_to_the_dollar() {
        // 16452 × 1.02 = 16781.04 → nearest dollar 16781 (formula check, not a CRA year).
        let indexed = index_claim_amount(money("16452.00"), Some(rate("0.020"))).unwrap();
        assert_eq!(indexed, money("16781"));
    }
}
