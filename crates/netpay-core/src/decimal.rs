//! Fixed-point decimal newtypes for CRA T4127 money, rates, and ratios.
//!
//! See `docs/ADR-001-decimal.md`. Constructors are lexical-string only.

use rust_decimal::Decimal;
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Mul, Neg};
use std::str::FromStr;
use thiserror::Error;

/// Errors from lexical parse and checked decimal arithmetic.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecimalError {
    /// Input failed the lexical character scan (or `Decimal` rejected it).
    #[error("invalid decimal format: {input}")]
    InvalidFormat { input: String },
    /// Checked arithmetic overflowed the 96-bit decimal mantissa.
    #[error("decimal arithmetic overflow")]
    Overflow,
    /// Division by zero.
    #[error("division by zero")]
    DivideByZero,
    /// More than 28 significant digits — would truncate inside `rust_decimal`.
    #[error("decimal precision overflow (more than 28 significant digits)")]
    PrecisionOverflow,
}

/// Monetary amount. Unbounded internal scale; `to_string` always emits 2dp.
///
/// Cent enforcement is `rounding.rs`, not this type (ADR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(Decimal);

/// Tax rate or similar coefficient (e.g. `0.0595`). Retains parse scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rate(Decimal);

/// Unrounded ratio from division (T4127 Chapter 6). Retains full internal precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ratio(Decimal);

/// Scan before `Decimal::from_str`: optional `-`, digits, at most one `.`, nothing else.
fn validate_lexical(input: &str) -> Result<(), DecimalError> {
    if input.is_empty() {
        return Err(DecimalError::InvalidFormat {
            input: input.to_string(),
        });
    }

    let bytes = input.as_bytes();
    let mut idx = 0usize;

    if bytes[idx] == b'-' {
        idx += 1;
        if idx == bytes.len() {
            return Err(DecimalError::InvalidFormat {
                input: input.to_string(),
            });
        }
    }

    let mut saw_digit = false;
    let mut saw_dot = false;
    let mut digit_count = 0u32;

    while idx < bytes.len() {
        match bytes[idx] {
            b'0'..=b'9' => {
                saw_digit = true;
                digit_count += 1;
            }
            b'.' if !saw_dot => {
                saw_dot = true;
            }
            _ => {
                return Err(DecimalError::InvalidFormat {
                    input: input.to_string(),
                });
            }
        }
        idx += 1;
    }

    if !saw_digit {
        return Err(DecimalError::InvalidFormat {
            input: input.to_string(),
        });
    }

    if digit_count > 28 {
        return Err(DecimalError::PrecisionOverflow);
    }

    Ok(())
}

fn parse_decimal(input: &str) -> Result<Decimal, DecimalError> {
    validate_lexical(input)?;
    Decimal::from_str(input).map_err(|_| DecimalError::InvalidFormat {
        input: input.to_string(),
    })
}

/// Format with exactly two fractional digits (pad when short; trunc when longer).
///
/// Truncation here is display-only. Payable cents go through `rounding.rs`.
fn format_two_decimals(value: Decimal) -> String {
    let value = if value.is_zero() {
        Decimal::ZERO
    } else if value.scale() > 2 {
        value.trunc_with_scale(2)
    } else {
        value
    };

    let s = value.to_string();
    match s.split_once('.') {
        None => format!("{s}.00"),
        Some((whole, frac)) => {
            let mut frac = frac.to_string();
            while frac.len() < 2 {
                frac.push('0');
            }
            format!("{whole}.{frac}")
        }
    }
}

impl Money {
    /// Additive identity.
    pub const ZERO: Money = Money(Decimal::ZERO);

    /// Parse money from lexical decimal form (ADR-001 / string-only constructors).
    pub fn parse(input: &str) -> Result<Money, DecimalError> {
        Ok(Money(parse_decimal(input)?))
    }

    pub(crate) fn from_decimal(value: Decimal) -> Money {
        Money(value)
    }

    pub(crate) fn as_decimal(self) -> Decimal {
        self.0
    }

    /// True when the value is strictly less than zero.
    pub fn is_negative(self) -> bool {
        self.0.is_sign_negative() && !self.0.is_zero()
    }

    /// True when the value is zero (including normalised negative zero).
    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    /// Checked addition.
    pub fn checked_add(self, rhs: Money) -> Result<Money, DecimalError> {
        self.0
            .checked_add(rhs.0)
            .map(Money)
            .ok_or(DecimalError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Money) -> Result<Money, DecimalError> {
        self.0
            .checked_sub(rhs.0)
            .map(Money)
            .ok_or(DecimalError::Overflow)
    }

    /// Checked multiply by another `Money` (overflow probe / rare paths).
    ///
    /// Prefer `checked_mul_rate` / `Mul<Rate>` / `Mul<Ratio>` in formulas;
    /// `Money * Money` via the `Mul` trait is intentionally absent.
    pub fn checked_mul(self, rhs: Money) -> Result<Money, DecimalError> {
        self.0
            .checked_mul(rhs.0)
            .map(Money)
            .ok_or(DecimalError::Overflow)
    }

    /// Checked multiply by a [`Rate`].
    pub fn checked_mul_rate(self, rhs: Rate) -> Result<Money, DecimalError> {
        self.0
            .checked_mul(rhs.0)
            .map(Money)
            .ok_or(DecimalError::Overflow)
    }

    /// Checked multiply by a [`Ratio`].
    pub fn checked_mul_ratio(self, rhs: Ratio) -> Result<Money, DecimalError> {
        self.0
            .checked_mul(rhs.0)
            .map(Money)
            .ok_or(DecimalError::Overflow)
    }

    /// Unrounded division → [`Ratio`] at full internal precision.
    pub fn checked_div(self, divisor: Money) -> Result<Ratio, DecimalError> {
        if divisor.is_zero() {
            return Err(DecimalError::DivideByZero);
        }
        self.0
            .checked_div(divisor.0)
            .map(Ratio)
            .ok_or(DecimalError::Overflow)
    }

    /// Component-wise maximum.
    pub fn max(self, other: Money) -> Money {
        Money(self.0.max(other.0))
    }

    /// Component-wise minimum.
    pub fn min(self, other: Money) -> Money {
        Money(self.0.min(other.0))
    }

    /// `max(0, self)` — T4127 non-negative clamp.
    pub fn floor_at_zero(self) -> Money {
        if self.is_negative() {
            Money::ZERO
        } else {
            self
        }
    }

    /// Fractional decimals retained internally.
    pub fn scale(&self) -> u32 {
        self.0.scale()
    }

    /// Compare money against an unrounded ratio (exact decimal equality).
    pub fn cmp_ratio(&self, other: &Ratio) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl Rate {
    /// Parse a rate from lexical decimal form; scale is preserved.
    pub fn parse(input: &str) -> Result<Rate, DecimalError> {
        Ok(Rate(parse_decimal(input)?))
    }

    /// Fractional decimals retained (trailing zeros kept).
    pub fn scale(&self) -> u32 {
        self.0.scale()
    }
}

impl Ratio {
    pub(crate) fn as_decimal(self) -> Decimal {
        self.0
    }

    /// Parse a ratio from lexical decimal form.
    pub fn parse(input: &str) -> Result<Ratio, DecimalError> {
        Ok(Ratio(parse_decimal(input)?))
    }

    /// `numerator / denominator` at full internal precision (T4127 Chapter 6).
    pub fn div(numerator: &str, denominator: &str) -> Result<Ratio, DecimalError> {
        let num = parse_decimal(numerator)?;
        let den = parse_decimal(denominator)?;
        if den.is_zero() {
            return Err(DecimalError::DivideByZero);
        }
        num.checked_div(den)
            .map(Ratio)
            .ok_or(DecimalError::Overflow)
    }

    /// Scale retained after division.
    pub fn scale(&self) -> u32 {
        self.0.scale()
    }

    /// Significant fractional decimals (uses retained scale as the precision signal).
    pub fn significant_decimals(&self) -> u32 {
        self.0.scale()
    }
}

impl Neg for Money {
    type Output = Money;

    fn neg(self) -> Money {
        Money(self.0.neg())
    }
}

impl Mul<Rate> for Money {
    type Output = Money;

    fn mul(self, rhs: Rate) -> Money {
        self.checked_mul_rate(rhs)
            .expect("Money * Rate overflow; use checked_mul_rate")
    }
}

impl Mul<Ratio> for Money {
    type Output = Money;

    fn mul(self, rhs: Ratio) -> Money {
        self.checked_mul_ratio(rhs)
            .expect("Money * Ratio overflow; use checked_mul_ratio")
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format_two_decimals(self.0))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_two_decimals(self.0))
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Display for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::{DecimalError, Money, Rate, Ratio};
    use std::cmp::Ordering;

    // --- PARSING -----------------------------------------------------------

    /// Round-trip lexical money form.
    #[test]
    fn parse_money_round_trips_two_decimal_string() {
        let m = Money::parse("1000.00").unwrap();
        assert_eq!(m.to_string(), "1000.00");
    }

    /// Rates must keep trailing scale; normalisation would drop tax-table precision.
    #[test]
    fn parse_rate_preserves_scale_four() {
        let r = Rate::parse("0.1400").unwrap();
        assert_eq!(r.scale(), 4);
    }

    /// Lexical CRA table values parse exactly (no float path).
    #[test]
    fn parse_money_and_rate_exact_for_cra_table_literals() {
        assert_eq!(Money::parse("74600.00").unwrap().to_string(), "74600.00");
        assert_eq!(Money::parse("4230.45").unwrap().to_string(), "4230.45");
        let rate = Rate::parse("0.00430").unwrap();
        assert_eq!(rate.scale(), 5);
        assert_eq!(rate.to_string(), "0.00430");
    }

    /// Reject empty, junk, multi-dot, scientific, underscores, currency, and commas.
    #[test]
    fn parse_rejects_non_lexical_decimal_forms() {
        for s in ["", "abc", "1.2.3", "1e5", "1_000", "$1000", "1,000.00"] {
            assert!(
                Money::parse(s).is_err(),
                "expected Err for {s:?} (scientific/float-shaped forms rejected deliberately)"
            );
        }
    }

    /// Negative zero is zero; sign bit must not report negative.
    #[test]
    fn parse_negative_zero_is_zero_and_not_negative() {
        let m = Money::parse("-0.00").unwrap();
        assert!(m.is_zero());
        assert!(!m.is_negative());
    }

    /// Over-precision input errors instead of silent truncation past rust_decimal's ceiling.
    #[test]
    fn parse_rejects_thirty_significant_digits_without_truncating() {
        let s = "123456789012345678901234567890"; // 30 significant digits
        assert!(matches!(
            Money::parse(s),
            Err(DecimalError::PrecisionOverflow)
        ));
    }

    // --- EXACTNESS (would fail under binary64) -----------------------------

    /// Classic decimal sum that binary64 cannot represent exactly.
    #[test]
    fn add_one_tenth_and_two_tenths_equals_three_tenths_exactly() {
        let sum = Money::parse("0.1")
            .unwrap()
            .checked_add(Money::parse("0.2").unwrap())
            .unwrap();
        assert_eq!(sum, Money::parse("0.3").unwrap());
        assert_eq!(sum.to_string(), "0.30");
    }

    /// Guards the binary64 0.8200000000000001 bug: 41/50 must compare Equal to 0.82.
    #[test]
    fn compare_point_eight_two_equals_forty_one_over_fifty() {
        let m = Money::parse("0.82").unwrap();
        assert_eq!(m.cmp(&Money::parse("0.82").unwrap()), Ordering::Equal);
        let ratio = Ratio::div("41", "50").unwrap();
        assert_eq!(m.cmp_ratio(&ratio), Ordering::Equal);
    }

    /// Repeated cent adds must not accumulate binary error.
    #[test]
    fn sum_one_cent_one_hundred_times_equals_one_dollar() {
        let cent = Money::parse("0.01").unwrap();
        let mut acc = Money::ZERO;
        for _ in 0..100 {
            acc = acc.checked_add(cent).unwrap();
        }
        assert_eq!(acc, Money::parse("1.00").unwrap());
        assert_eq!(acc.to_string(), "1.00");
    }

    /// GST-style rate multiply stays exact at the cent.
    #[test]
    fn mul_money_by_rate_exact_at_cent() {
        let product = Money::parse("1000.00")
            .unwrap()
            .checked_mul_rate(Rate::parse("0.0595").unwrap())
            .unwrap();
        assert_eq!(product, Money::parse("59.50").unwrap());
        assert_eq!(product.to_string(), "59.50");
    }

    /// Division stays unrounded; cent rounding is rounding.rs's job, not checked_div.
    #[test]
    fn checked_div_retains_unrounded_intermediate() {
        let q = Money::parse("3500.00")
            .unwrap()
            .checked_div(Money::parse("12").unwrap())
            .unwrap();
        // Must not already be the cent-rounded 291.67 / 291.66.
        assert_ne!(q.to_string(), "291.67");
        assert_ne!(q.to_string(), "291.66");
        assert!(
            q.scale() >= 20 || q.significant_decimals() >= 20,
            "expected >= 20 significant decimals of unrounded quotient, got scale={} sig={}",
            q.scale(),
            q.significant_decimals()
        );
    }

    /// Decimal addition associativity for values that bite binary floats.
    #[test]
    fn add_is_associative_for_point_zero_seven_one_and_seven() {
        let a = Money::parse("0.07").unwrap();
        let b = Money::parse("0.1").unwrap();
        let c = Money::parse("0.7").unwrap();
        let left = a.checked_add(b).unwrap().checked_add(c).unwrap();
        let right = a.checked_add(b.checked_add(c).unwrap()).unwrap();
        assert_eq!(left, right);
    }

    // --- RATIOS (never rounded, T4127 Chapter 6) ---------------------------

    /// CRA worked example §6.13: 0.0100/0.0595 × 204.25 → 34.33 after half-up to cent.
    #[test]
    fn ratio_cra_6_13_employment_insurance_premium_reduction() {
        let ratio = Ratio::div("0.0100", "0.0595").unwrap();
        let product = Money::parse("204.25").unwrap() * ratio;
        let rounded = crate::rounding::round_half_up_to_cent(product);
        assert_eq!(rounded.to_string(), "34.33");
    }

    /// Chapter 6 ratios keep full intermediate scale (never pre-rounded).
    #[test]
    fn ratio_div_keeps_scale_at_least_twenty() {
        let r = Ratio::div("0.0495", "0.0595").unwrap();
        assert!(r.scale() >= 20);
    }

    /// Divide-by-zero is Err(DivideByZero), never panic or infinity.
    #[test]
    fn ratio_div_by_zero_returns_typed_error() {
        assert!(matches!(
            Ratio::div("0.0595", "0.0000"),
            Err(DecimalError::DivideByZero)
        ));
    }

    // --- SIGN AND SATURATION -----------------------------------------------

    /// Negative money floors to zero for T4127 max(0, ·) style clamps.
    #[test]
    fn floor_at_zero_clamps_negatives_only() {
        assert_eq!(
            Money::parse("-5.00").unwrap().floor_at_zero().to_string(),
            "0.00"
        );
        assert_eq!(
            Money::parse("5.00").unwrap().floor_at_zero().to_string(),
            "5.00"
        );
    }

    // Compile-fail coverage for no IEEE-754 From/Into conversions lives in
    // `tests/compile_fail/no_float.rs`. Wiring it requires `trybuild` — ask
    // before adding that dependency (see Step 1). Uncomment when approved:
    //
    // #[test]
    // fn money_has_no_float_conversions_compile_fail() {
    //     let t = trybuild::TestCases::new();
    //     t.compile_fail("tests/compile_fail/no_float.rs");
    // }

    /// Overflowing multiply returns Err, never panic or wrap.
    #[test]
    fn checked_mul_overflow_returns_err() {
        let huge = Money::parse("9999999999999999999999999999").unwrap();
        assert!(matches!(
            huge.checked_mul(huge),
            Err(DecimalError::Overflow)
        ));
    }

    // --- FORMATTING --------------------------------------------------------

    /// Money keeps unbounded internal scale; display is always 2dp; cents are rounding.rs.
    #[test]
    fn money_display_two_decimals_parse_retains_unbounded_scale() {
        assert_eq!(Money::parse("5").unwrap().to_string(), "5.00");
        assert_eq!(Money::parse("5.1").unwrap().to_string(), "5.10");
        let sub_cent = Money::parse("5.005").unwrap();
        assert_eq!(sub_cent.scale(), 3);
        // Display is always exactly two fractional digits (padding when scale < 2).
        // Sub-cent values are still Money; callers round via rounding.rs before
        // treating the value as a payable cent amount.
        assert_eq!(
            sub_cent.to_string().split('.').nth(1).map(|f| f.len()),
            Some(2)
        );
    }

    // --- OPERATORS / HELPERS (surface smoke) -------------------------------

    /// Negation and checked_sub are available and exact.
    #[test]
    fn money_neg_and_checked_sub() {
        let m = Money::parse("5.00").unwrap();
        assert_eq!((-m).to_string(), "-5.00");
        assert_eq!(
            m.checked_sub(Money::parse("2.50").unwrap())
                .unwrap()
                .to_string(),
            "2.50"
        );
    }

    /// max/min select without altering scale of the chosen value.
    #[test]
    fn money_max_min() {
        let a = Money::parse("1.00").unwrap();
        let b = Money::parse("2.00").unwrap();
        assert_eq!(a.max(b).to_string(), "2.00");
        assert_eq!(a.min(b).to_string(), "1.00");
    }

    /// Rate multiply via Mul<Rate> matches checked_mul_rate.
    #[test]
    fn money_mul_rate_operator() {
        let m = Money::parse("100.00").unwrap();
        let r = Rate::parse("0.0500").unwrap();
        assert_eq!((m * r).to_string(), "5.00");
    }

    // --- SERDE -------------------------------------------------------------

    /// Money serializes as a JSON string, never a JSON number.
    #[test]
    fn money_serde_json_is_string_not_number() {
        let money = Money::parse("1000.00").unwrap();
        assert_eq!(serde_json::to_string(&money).unwrap(), "\"1000.00\"");
    }
}
