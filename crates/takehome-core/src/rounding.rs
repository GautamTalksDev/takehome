//! Named rounding functions for CRA T4127 payroll deduction formulas.
//!
//! Every rounding decision in takehome-core is an explicit call into this module.
//! Callers pass an already-divided [`Ratio`]; these functions never divide.
//!
//! | Function | Rule | CRA locus |
//! |---|---|---|
//! | [`round_tax_to_cent`] | Half-up at the cent | Income tax deduction |
//! | [`round_contribution_to_cent`] | Half-up at the cent | CPP, CPP2, EI, QPIP |
//! | [`truncate_exemption_to_cent`] | Drop the third decimal (no half-up) | CPP basic exemption / P |
//! | [`round_bpa_to_cent`] | Half-up at the cent | BPAF / BPAMB / BPAYT |
//! | [`round_claim_to_dollar`] | Half-up to the nearest dollar | TD1 claim indexing |
//! | [`round_final`] | Cent identity, or half-up to the nickel | PDOC remittance granularity |
//!
//! Source: [T4127 Payroll Deductions Formulas](https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html)

use rust_decimal::{Decimal, RoundingStrategy};

use crate::decimal::{Money, Ratio};

/// PDOC final remittance granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    /// Default: already at the cent; identity.
    Cent,
    /// Round to the nearest five cents.
    Nickel,
}

fn assert_non_negative_for_formula_step(value: Ratio, api: &'static str) {
    let negative = is_strictly_negative(value);
    // Unit tests deliberately exercise the negative edge; production debug
    // builds still treat a negative input as a caller bug.
    #[cfg(not(test))]
    debug_assert!(!negative, "negative input to {api} is a caller bug");
    let _ = (negative, api);
}

fn is_strictly_negative(value: Ratio) -> bool {
    let d = value.as_decimal();
    d.is_sign_negative() && !d.is_zero()
}

/// Income tax deduction: *"increase the second digit … if the third digit is five or more"*.
///
/// T4127 Chapter 2 / Rounding procedures — For income tax deductions.
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from [`truncate_exemption_to_cent`]: tax uses half-up, not drop-the-third.
pub fn round_tax_to_cent(value: Ratio) -> Money {
    // Do NOT use Decimal::round() / round_dp without a strategy — the default is
    // MidpointNearestEven (banker's rounding), the single most likely wrong default here.
    Money::from_decimal(
        value
            .as_decimal()
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    )
}

/// CPP, CPP2, EI, QPIP contribution: *"increase the second digit … if the third digit is five or more"*.
///
/// T4127 Chapter 2 / Rounding procedures — For Canada Pension Plan contributions
/// (and the parallel EI / QPIP wording).
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from [`truncate_exemption_to_cent`]: same half-up as tax, never truncation.
/// Kept as its own function so call sites name the CRA contribution rule explicitly.
pub fn round_contribution_to_cent(value: Ratio) -> Money {
    assert_non_negative_for_formula_step(value, "round_contribution_to_cent");

    // Do NOT use Decimal::round() — default is banker's rounding (MidpointNearestEven).
    Money::from_decimal(
        value
            .as_decimal()
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    )
}

/// CPP basic exemption per pay period: *"drop the third digit after the decimal point"*.
///
/// T4127 Chapter 2 / Rounding procedures — For Canada Pension Plan basic exemption
/// (table values in Chapter 6).
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from [`round_contribution_to_cent`]: this DROPS the third decimal; it does
/// not half-up. The 291.66 vs 291.67 canary lives here.
pub fn truncate_exemption_to_cent(value: Ratio) -> Money {
    assert_non_negative_for_formula_step(value, "truncate_exemption_to_cent");

    // Explicit truncation — not a rounding mode that happens to truncate.
    Money::from_decimal(value.as_decimal().trunc_with_scale(2))
}

/// BPAF / BPAMB / BPAYT: *"increase the second digit … if the third digit is five or more"*.
///
/// T4127 Chapter 2 — Personal tax credits / Basic Personal Amount formulas.
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from neighbours only in call-site meaning: the internal BPA division that
/// produced `value` must remain unrounded; this function is the sole rounding step.
pub fn round_bpa_to_cent(value: Ratio) -> Money {
    // Do NOT use Decimal::round() — default is banker's rounding (MidpointNearestEven).
    Money::from_decimal(
        value
            .as_decimal()
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    )
}

/// TD1 claim indexing: *"rounded to the nearest dollar"*.
///
/// T4127 Chapter 2 — Option 1 Indexing of Personal Amounts.
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from the cent half-up rules: granularity is the whole dollar, not the cent.
pub fn round_claim_to_dollar(value: Ratio) -> Money {
    // Do NOT use Decimal::round() — default is banker's rounding (MidpointNearestEven).
    Money::from_decimal(
        value
            .as_decimal()
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero),
    )
}

/// PDOC remittance granularity: cent identity, or half-up to the nickel.
///
/// T4127 / PDOC remittance behaviour (cent default; optional nickel).
/// <https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jan/t4127-jan-payroll-deductions-formulas-computer-programs.html>
///
/// Differs from formula-step rounders: this is the final remittance quantum, not a
/// Chapter 2 tax/contribution step.
pub fn round_final(value: Money, granularity: Granularity) -> Money {
    match granularity {
        Granularity::Cent => value,
        Granularity::Nickel => {
            // Scale to twentieths of a dollar, half-up to an integer count of nickels,
            // then scale back by multiplying by 0.05 (no division).
            // Do NOT use Decimal::round() — default is banker's rounding.
            let twentieths = value.as_decimal() * Decimal::from(20u32);
            let nickels =
                twentieths.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
            let nickel = Decimal::new(5, 2); // 0.05 from integer mantissa — not a float
            Money::from_decimal(nickels * nickel)
        }
    }
}

/// Half-up at the cent for an already-computed [`Money`] intermediate.
///
/// Used by the Chapter 6.13 decimal worked example. Same numeric rule as
/// [`round_tax_to_cent`] / [`round_contribution_to_cent`], kept so Money-typed
/// call sites do not round-trip through display strings.
pub fn round_half_up_to_cent(value: Money) -> Money {
    // Do NOT use Decimal::round() — default is banker's rounding (MidpointNearestEven).
    Money::from_decimal(
        value
            .as_decimal()
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    )
}
#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Ratio};
    use crate::rounding::{
        round_bpa_to_cent, round_claim_to_dollar, round_contribution_to_cent, round_final,
        round_tax_to_cent, truncate_exemption_to_cent, Granularity,
    };
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use rust_decimal::RoundingStrategy;
    use std::cmp::Ordering;
    use std::str::FromStr;

    /// T4127 Chapter 6 — annual CPP basic exemption divided by pay periods.
    const EXEMPTION_TABLE: &[(&str, &str)] = &[
        ("1", "3500.00"),
        ("2", "1750.00"),
        ("4", "875.00"),
        ("10", "350.00"),
        ("12", "291.66"),
        ("13", "269.23"),
        ("22", "159.09"),
        ("24", "145.83"),
        ("26", "134.61"),
        ("27", "129.62"),
        ("52", "67.30"),
        ("53", "66.03"),
        ("240", "14.58"),
        ("2000", "1.75"),
    ];

    fn ratio(s: &str) -> Ratio {
        Ratio::parse(s).unwrap_or_else(|e| panic!("Ratio::parse({s:?}): {e}"))
    }

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap_or_else(|e| panic!("Money::parse({s:?}): {e}"))
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap_or_else(|e| panic!("Decimal::from_str({s:?}): {e}"))
    }

    fn ratio_decimal(r: Ratio) -> Decimal {
        dec(&r.to_string())
    }

    /// Ordinary half-up at two decimals (away from zero at midpoint).
    fn half_up_to_cent(d: Decimal) -> Decimal {
        d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
    }

    /// CRA Chapter 2 wording literally: truncate to 3dp, then if 3rd digit ≥ 5
    /// increase the 2nd decimal by one (away from zero); else drop the 3rd.
    fn cra_literal_wording_to_cent(d: Decimal) -> Decimal {
        let truncated_3 = d.trunc_with_scale(3);
        let third_digit = third_decimal_digit(truncated_3);
        let truncated_2 = d.trunc_with_scale(2);
        if third_digit >= 5 {
            let bump = if d.is_sign_negative() && !d.is_zero() {
                -dec("0.01")
            } else {
                dec("0.01")
            };
            truncated_2 + bump
        } else {
            truncated_2
        }
    }

    /// Third fractional digit of a value already truncated to ≤3 decimal places.
    fn third_decimal_digit(d: Decimal) -> u32 {
        let s = d.abs().to_string();
        match s.split_once('.') {
            None => 0,
            Some((_, frac)) => {
                let bytes = frac.as_bytes();
                if bytes.len() >= 3 {
                    u32::from(bytes[2] - b'0')
                } else {
                    0
                }
            }
        }
    }

    /// Lexical decimal strings with 3–10 fractional digits, magnitude in ±1e6.
    /// Built from integers only — never passes through IEEE-754 binary floats.
    fn lexical_decimal_strategy() -> impl Strategy<Value = String> {
        (
            -1_000_000i64..=1_000_000i64,
            prop::collection::vec(0u8..=9u8, 3..=10),
        )
            .prop_filter(
                "exclude i64::MIN magnitude edge for abs formatting",
                |(w, _)| *w != i64::MIN,
            )
            .prop_map(|(whole, digits)| {
                let sign = if whole < 0 { "-" } else { "" };
                let abs = whole.unsigned_abs();
                let frac: String = digits.into_iter().map(|d| char::from(b'0' + d)).collect();
                format!("{sign}{abs}.{frac}")
            })
    }

    fn ratio_strategy() -> impl Strategy<Value = Ratio> {
        lexical_decimal_strategy().prop_filter_map("parseable ratio", |s| Ratio::parse(&s).ok())
    }

    // --- THE 291.66 CANARY -------------------------------------------------

    /// T4127 Ch.6 CPP basic exemption / monthly: truncation, not half-up.
    /// 3500/12 = 291.6666…; half-up would be 291.67. CRA publishes 291.66.
    /// If this ever expects 291.67 the engine is wrong for every monthly-paid employee.
    #[test]
    fn truncate_exemption_monthly_is_291_66_not_291_67() {
        let input = Ratio::div("3500.00", "12").unwrap();
        assert_eq!(truncate_exemption_to_cent(input).to_string(), "291.66");
    }

    /// T4127 Ch.2 contribution half-up on the same 3500/12 input → 291.67.
    /// Discriminating pair: proves truncate and contribution are not aliases.
    #[test]
    fn contribution_half_up_on_same_input_is_291_67() {
        let input = Ratio::div("3500.00", "12").unwrap();
        assert_eq!(round_contribution_to_cent(input).to_string(), "291.67");
        assert_ne!(
            truncate_exemption_to_cent(input).to_string(),
            round_contribution_to_cent(input).to_string()
        );
    }

    // --- FULL EXEMPTION TABLE ----------------------------------------------

    /// T4127 Chapter 6 — CPP basic exemption per pay period (full legal table).
    /// Load-bearing canaries (truncation ≠ half-up): P=12,27,52,53.
    #[test]
    fn truncate_exemption_full_t4127_chapter6_table() {
        for &(p, expected) in EXEMPTION_TABLE {
            let got = truncate_exemption_to_cent(Ratio::div("3500.00", p).unwrap());
            assert_eq!(
                got.to_string(),
                expected,
                "P={p}: truncate_exemption_to_cent(3500/{p})"
            );
        }

        // Explicit discriminating rows (truncation vs half-up disagree).
        let disagree: &[(&str, &str, &str)] = &[
            ("12", "291.66", "291.67"),
            ("27", "129.62", "129.63"),
            ("52", "67.30", "67.31"),
            ("53", "66.03", "66.04"),
        ];
        for &(p, trunc, half) in disagree {
            let input = Ratio::div("3500.00", p).unwrap();
            assert_eq!(truncate_exemption_to_cent(input).to_string(), trunc);
            assert_eq!(round_contribution_to_cent(input).to_string(), half);
        }
    }

    // --- HALF-UP BOUNDARY BEHAVIOUR ----------------------------------------

    /// T4127 Ch.2 tax half-up: third decimal 5 bumps. Binary64 stores 1.005
    /// as 1.004999… and naive rounding wrongly yields 1.00 — second float canary.
    #[test]
    fn round_tax_one_point_zero_zero_five_is_one_point_zero_one() {
        assert_eq!(round_tax_to_cent(ratio("1.005")).to_string(), "1.01");
    }

    /// T4127 Ch.2 — third decimal 4 drops.
    #[test]
    fn round_tax_one_point_zero_zero_four_is_one_point_zero_zero() {
        assert_eq!(round_tax_to_cent(ratio("1.004")).to_string(), "1.00");
    }

    /// T4127 Ch.2 — after truncating attention to the third decimal, digit is 4.
    #[test]
    fn round_tax_one_point_zero_zero_four_repeating_stays_one_point_zero_zero() {
        assert_eq!(round_tax_to_cent(ratio("1.0049999")).to_string(), "1.00");
    }

    /// T4127 Ch.2 — third decimal effectively 5 with a positive remainder.
    #[test]
    fn round_tax_one_point_zero_zero_five_plus_epsilon_is_one_point_zero_one() {
        assert_eq!(round_tax_to_cent(ratio("1.0050001")).to_string(), "1.01");
    }

    /// T4127 Ch.2 — classic binary64 half-up failure case (2.675 → 2.68).
    #[test]
    fn round_tax_two_point_six_seven_five_is_two_point_six_eight() {
        assert_eq!(round_tax_to_cent(ratio("2.675")).to_string(), "2.68");
    }

    /// T4127 Ch.2 — half-up from half a cent.
    #[test]
    fn round_tax_zero_point_zero_zero_five_is_zero_point_zero_one() {
        assert_eq!(round_tax_to_cent(ratio("0.005")).to_string(), "0.01");
    }

    /// T4127 Ch.2 — drop; display must be "0.00" not "0".
    #[test]
    fn round_tax_zero_point_zero_zero_four_is_zero_point_zero_zero() {
        let m = round_tax_to_cent(ratio("0.004"));
        assert_eq!(m.to_string(), "0.00");
        assert_ne!(m.to_string(), "0");
    }

    // --- LITERAL-CRA-WORDING EQUIVALENCE -----------------------------------

    /// T4127 Ch.2 wording ("look at the third decimal") ≡ ordinary half-up at 2dp.
    /// If this property fails, the CRA rule is doing something we do not understand.
    #[test]
    fn cra_chapter2_wording_equivalent_to_half_up_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        runner
            .run(&lexical_decimal_strategy(), |s| {
                let Ok(d) = Decimal::from_str(&s) else {
                    return Ok(());
                };
                let literal = cra_literal_wording_to_cent(d);
                let ordinary = half_up_to_cent(d);
                prop_assert_eq!(literal, ordinary, "disagree on {}", s);

                // Engine entry point must match the proven-equivalent rule.
                let via_api = round_tax_to_cent(Ratio::parse(&s).unwrap());
                prop_assert_eq!(via_api, Money::from_decimal(ordinary));
                Ok(())
            })
            .unwrap();
    }

    // --- NEGATIVES ---------------------------------------------------------

    /// T4127 never produces negative tax (floors at zero). Pin AWAY FROM ZERO
    /// (-1.01). A negative input is a caller bug; the engine should never reach this path.
    /// Implementation (step 4) must `debug_assert` non-negative on
    /// `round_contribution_to_cent` and `truncate_exemption_to_cent`.
    #[test]
    fn round_tax_negative_half_away_from_zero() {
        assert_eq!(round_tax_to_cent(ratio("-1.005")).to_string(), "-1.01");
    }

    /// Truncation toward zero on negatives (same caller-bug rationale as tax).
    #[test]
    fn truncate_exemption_negative_toward_zero() {
        assert_eq!(
            truncate_exemption_to_cent(ratio("-291.666")).to_string(),
            "-291.66"
        );
    }

    // --- NICKEL ROUNDING ---------------------------------------------------

    /// PDOC nickel granularity — 10.02 → 10.00.
    #[test]
    fn round_final_nickel_10_02_to_10_00() {
        assert_eq!(
            round_final(money("10.02"), Granularity::Nickel).to_string(),
            "10.00"
        );
    }

    /// PDOC nickel granularity — 10.03 → 10.05.
    #[test]
    fn round_final_nickel_10_03_to_10_05() {
        assert_eq!(
            round_final(money("10.03"), Granularity::Nickel).to_string(),
            "10.05"
        );
    }

    /// PDOC nickel half-up — 10.025 → 10.05.
    #[test]
    fn round_final_nickel_10_025_to_10_05() {
        assert_eq!(
            round_final(money("10.025"), Granularity::Nickel).to_string(),
            "10.05"
        );
    }

    /// Default PDOC path is cent granularity — identity at the cent.
    #[test]
    fn round_final_cent_is_identity() {
        assert_eq!(
            round_final(money("10.02"), Granularity::Cent).to_string(),
            "10.02"
        );
    }

    // --- IDEMPOTENCE AND INVARIANTS (proptest) -----------------------------

    /// T4127 Ch.2 — every named rounder is idempotent once at cent/dollar scale.
    #[test]
    fn all_rounders_are_idempotent_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        runner
            .run(&ratio_strategy(), |x| {
                let tax = round_tax_to_cent(x);
                prop_assert_eq!(
                    tax,
                    round_tax_to_cent(Ratio::parse(&tax.to_string()).unwrap())
                );

                let contrib = round_contribution_to_cent(x);
                prop_assert_eq!(
                    contrib,
                    round_contribution_to_cent(Ratio::parse(&contrib.to_string()).unwrap())
                );

                let trunc = truncate_exemption_to_cent(x);
                prop_assert_eq!(
                    trunc,
                    truncate_exemption_to_cent(Ratio::parse(&trunc.to_string()).unwrap())
                );

                let bpa = round_bpa_to_cent(x);
                prop_assert_eq!(
                    bpa,
                    round_bpa_to_cent(Ratio::parse(&bpa.to_string()).unwrap())
                );

                let claim = round_claim_to_dollar(x);
                prop_assert_eq!(
                    claim,
                    round_claim_to_dollar(Ratio::parse(&claim.to_string()).unwrap())
                );

                let cent = round_final(tax, Granularity::Cent);
                prop_assert_eq!(cent, round_final(cent, Granularity::Cent));

                let nickel = round_final(tax, Granularity::Nickel);
                prop_assert_eq!(nickel, round_final(nickel, Granularity::Nickel));
                Ok(())
            })
            .unwrap();
    }

    /// T4127 Ch.2 — tax half-up never moves a value by more than half a cent.
    #[test]
    fn round_tax_error_bound_half_cent_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        runner
            .run(&lexical_decimal_strategy(), |s| {
                let Ok(x) = Decimal::from_str(&s) else {
                    return Ok(());
                };
                let Ok(r) = Ratio::parse(&s) else {
                    return Ok(());
                };
                let rounded = dec(&round_tax_to_cent(r).to_string());
                let err = (rounded - x).abs();
                prop_assert!(err <= dec("0.005"), "|round_tax({s}) - x| = {err} > 0.005");
                Ok(())
            })
            .unwrap();
    }

    /// T4127 Ch.6 — truncation never increases a non-negative exemption.
    #[test]
    fn truncate_exemption_never_increases_non_negative_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        let strat = lexical_decimal_strategy().prop_filter_map("non-negative ratio", |s| {
            if s.starts_with('-') {
                None
            } else {
                Ratio::parse(&s).ok()
            }
        });
        runner
            .run(&strat, |x| {
                let t = truncate_exemption_to_cent(x);
                prop_assert!(
                    t.cmp_ratio(&x) != Ordering::Greater,
                    "truncate({})={} > x",
                    x,
                    t
                );
                Ok(())
            })
            .unwrap();
    }

    /// T4127 Ch.2 — all six rounders are monotone.
    #[test]
    fn all_rounders_are_monotone_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        runner
            .run(&(ratio_strategy(), ratio_strategy()), |(a, b)| {
                if ratio_decimal(a).cmp(&ratio_decimal(b)) == Ordering::Greater {
                    return Ok(());
                }
                // a <= b
                prop_assert!(round_tax_to_cent(a) <= round_tax_to_cent(b));
                prop_assert!(round_contribution_to_cent(a) <= round_contribution_to_cent(b));
                prop_assert!(truncate_exemption_to_cent(a) <= truncate_exemption_to_cent(b));
                prop_assert!(round_bpa_to_cent(a) <= round_bpa_to_cent(b));
                prop_assert!(round_claim_to_dollar(a) <= round_claim_to_dollar(b));
                prop_assert!(
                    round_final(round_tax_to_cent(a), Granularity::Cent)
                        <= round_final(round_tax_to_cent(b), Granularity::Cent)
                );
                prop_assert!(
                    round_final(round_tax_to_cent(a), Granularity::Nickel)
                        <= round_final(round_tax_to_cent(b), Granularity::Nickel)
                );
                Ok(())
            })
            .unwrap();
    }

    /// T4127 Ch.2 — cent rounders return scale 2; claim indexing returns scale 0.
    #[test]
    fn rounders_return_expected_money_scale_property() {
        let mut config = ProptestConfig::with_cases(10_000);
        config.source_file = Some(file!());
        let mut runner = proptest::test_runner::TestRunner::new(config);
        runner
            .run(&ratio_strategy(), |x| {
                prop_assert_eq!(round_tax_to_cent(x).scale(), 2);
                prop_assert_eq!(round_contribution_to_cent(x).scale(), 2);
                prop_assert_eq!(truncate_exemption_to_cent(x).scale(), 2);
                prop_assert_eq!(round_bpa_to_cent(x).scale(), 2);
                prop_assert_eq!(round_claim_to_dollar(x).scale(), 0);
                prop_assert_eq!(
                    round_final(round_tax_to_cent(x), Granularity::Cent).scale(),
                    2
                );
                prop_assert_eq!(
                    round_final(round_tax_to_cent(x), Granularity::Nickel).scale(),
                    2
                );
                Ok(())
            })
            .unwrap();
    }
}
