//! Per-period tax T (T4127 Chapter 4 step 6).
//!
//! ```text
//! T = [(T1 + T2) / P] + L
//! ```
//!
//! Rounded with [`round_tax_to_cent`](crate::rounding::round_tax_to_cent); L is
//! added after the division and is never rounded away.

use crate::decimal::{DecimalError, Money};
use crate::request::PayPeriod;
use crate::rounding::{round_final, round_tax_to_cent, Granularity};

/// Compute payable tax for one period. L is added after annual-tax division.
pub fn per_period_tax(
    t1: Money,
    t2: Money,
    pay_period: PayPeriod,
    additional_tax: Money,
    granularity: Granularity,
) -> Result<Money, DecimalError> {
    let annual = t1.checked_add(t2)?;
    let p = Money::parse(&pay_period.get().to_string())?;
    let base = round_tax_to_cent(annual.checked_div(p)?);
    Ok(round_final(
        base.checked_add(additional_tax)?.floor_at_zero(),
        granularity,
    ))
}

#[cfg(test)]
mod tests {
    use crate::decimal::Money;
    use crate::request::PayPeriod;
    use crate::rounding::{round_final, round_tax_to_cent, Granularity};

    use super::per_period_tax;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    /// 115. Step 6 rounds via `round_tax_to_cent`, PDOC granularity Cent.
    #[test]
    fn step6_uses_round_tax_to_cent_and_cent_granularity() {
        let p = PayPeriod::new(52).unwrap();
        let t1 = money("4243.86");
        let t2 = money("2381.51");
        // (4243.86 + 2381.51) / 52 = 127.410961… → 127.41
        let t = per_period_tax(t1, t2, p, money("0.00"), Granularity::Cent).unwrap();
        assert_eq!(t, money("127.41"));

        let ratio = t1
            .checked_add(t2)
            .unwrap()
            .checked_div(money("52"))
            .unwrap();
        assert_eq!(round_tax_to_cent(ratio), money("127.41"));
        assert_eq!(
            round_final(money("127.41"), Granularity::Cent),
            money("127.41")
        );
    }

    /// 116. Additional tax L is added AFTER the division and is never rounded away.
    #[test]
    fn additional_tax_l_added_after_division_never_rounded_away() {
        let p = PayPeriod::new(52).unwrap();
        // Choose T1+T2 so (T1+T2)/P rounds down toward a .xx4 that L must still appear on.
        // (100.00 + 0) / 52 = 1.92307… → 1.92; + L=0.01 → 1.93 (L survives).
        let without_l = per_period_tax(
            money("100.00"),
            money("0.00"),
            p,
            money("0.00"),
            Granularity::Cent,
        )
        .unwrap();
        assert_eq!(without_l, money("1.92"));

        let with_l = per_period_tax(
            money("100.00"),
            money("0.00"),
            p,
            money("0.01"),
            Granularity::Cent,
        )
        .unwrap();
        assert_eq!(with_l, money("1.93"));

        // L is not folded into the dividend (that would round 100.01/52 = 1.92326… → still 1.92).
        let wrongly_inside = round_tax_to_cent(money("100.01").checked_div(money("52")).unwrap());
        assert_eq!(wrongly_inside, money("1.92"));
        assert_ne!(
            with_l, wrongly_inside,
            "L must be added after rounding (T1+T2)/P, not inside the dividend"
        );
    }

    /// L of several dollars lands in full on top of a rounded quotient.
    #[test]
    fn large_l_is_preserved_in_full() {
        let t = per_period_tax(
            money("0.00"),
            money("0.00"),
            PayPeriod::new(26).unwrap(),
            money("25.00"),
            Granularity::Cent,
        )
        .unwrap();
        assert_eq!(t, money("25.00"));
    }
}
