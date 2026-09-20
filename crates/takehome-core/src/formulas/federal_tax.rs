//! Federal tax T3 / T1 (T4127 Chapter 4 steps 2–3).
//!
//! Bracket selection: highest threshold ≤ A; apply that bracket's R and K to
//! all of A (do not sum marginal slices — K exists so you do not have to).
//!
//! K is the **published** whole-dollar constant (ADR-002). The algebra
//! `(R × A) − K_exact = Σ slices` is off by at most $0.50 per bracket because
//! CRA rounds K to the dollar. That residual is a conformance asset.

use crate::decimal::{DecimalError, Money, Rate};
use crate::request::{PayPeriod, Province};
use crate::rounding::round_tax_to_cent;
use crate::rules::schema::{Bracket, LabourCredit};
use thiserror::Error;

/// Federal formula failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FederalTaxError {
    #[error("tax bracket table is empty or has no bracket for A")]
    MissingBracket,
    #[error(transparent)]
    Decimal(#[from] DecimalError),
}

/// Select the highest bracket whose threshold is at or below A.
pub fn select_federal_bracket(
    annual_taxable_income: Money,
    brackets: &[Bracket],
) -> Result<Bracket, FederalTaxError> {
    brackets
        .iter()
        .rev()
        .find(|bracket| annual_taxable_income >= bracket.threshold)
        .cloned()
        .ok_or(FederalTaxError::MissingBracket)
}

/// T3 = max(0, (R × A) − K − K1 − K2 − K3 − K4).
pub fn federal_t3(
    a: Money,
    k1: Money,
    k2: Money,
    k3: Money,
    k4: Money,
    brackets: &[Bracket],
) -> Result<Money, FederalTaxError> {
    let bracket = select_federal_bracket(a, brackets)?;
    let raw = a
        .checked_mul_rate(bracket.rate)?
        .checked_sub(bracket.constant)?
        .checked_sub(k1)?
        .checked_sub(k2)?
        .checked_sub(k3)?
        .checked_sub(k4)?
        .floor_at_zero();
    Ok(round_tax_to_cent(raw.checked_div(Money::parse("1")?)?))
}

fn labour_credit(purchase: Option<Money>, params: &LabourCredit) -> Result<Money, FederalTaxError> {
    let Some(purchase) = purchase else {
        return Ok(Money::ZERO);
    };
    let raw = purchase
        .floor_at_zero()
        .checked_mul_rate(params.rate)?
        .min(params.max);
    Ok(round_tax_to_cent(raw.checked_div(Money::parse("1")?)?))
}

/// Assemble T1, including LCF, Quebec abatement, or Outside-Canada surtax.
pub fn federal_t1(
    t3: Money,
    pay_periods: PayPeriod,
    lcf_purchase: Option<Money>,
    lcf_params: &LabourCredit,
    province: Province,
    quebec_abatement: Option<Rate>,
    outside_canada_surtax: Option<Rate>,
) -> Result<Money, FederalTaxError> {
    let p = Money::parse(&pay_periods.get().to_string())?;
    let lcf = labour_credit(lcf_purchase, lcf_params)?;
    let mut result = t3.checked_sub(lcf.checked_mul(p)?)?;
    if province == Province::Qc {
        if let Some(rate) = quebec_abatement {
            result = result.checked_sub(t3.checked_mul_rate(rate)?)?;
        }
    } else if province == Province::OutsideCanada {
        if let Some(rate) = outside_canada_surtax {
            result = result.checked_add(t3.checked_mul_rate(rate)?)?;
        }
    }
    let result = result.floor_at_zero();
    Ok(round_tax_to_cent(result.checked_div(Money::parse("1")?)?))
}

/// Provinces with no T2 path always return zero.
pub fn provincial_t2_or_zero(province: Province, computed: Money) -> Money {
    match province {
        Province::Qc | Province::OutsideCanada => Money::ZERO,
        _ => computed.floor_at_zero(),
    }
}

#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Rate};
    use crate::request::{PayPeriod, Province};
    use crate::rounding::round_tax_to_cent;
    use crate::rules::schema::{Bracket, LabourCredit};
    use proptest::prelude::*;

    use super::{federal_t1, federal_t3, provincial_t2_or_zero, select_federal_bracket};

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    /// 2026 federal Table 8.1 brackets (hand-authored rule data).
    fn federal_brackets() -> Vec<Bracket> {
        vec![
            Bracket {
                threshold: money("0"),
                rate: rate("0.1400"),
                constant: money("0.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("58523.00"),
                rate: rate("0.2050"),
                constant: money("3804.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("117045.00"),
                rate: rate("0.2600"),
                constant: money("10241.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("181440.00"),
                rate: rate("0.2900"),
                constant: money("15685.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("258482.00"),
                rate: rate("0.3300"),
                constant: money("26024.00"),
                prorated: false,
            },
        ]
    }

    fn lcf_params() -> LabourCredit {
        LabourCredit {
            rate: rate("0.150"),
            max: money("750.00"),
        }
    }

    /// 81. A = 50000 → bracket 1, R = 0.1400, K = 0
    #[test]
    fn bracket_below_first_threshold() {
        let b = select_federal_bracket(money("50000.00"), &federal_brackets()).unwrap();
        assert_eq!(b.rate, rate("0.1400"));
        assert_eq!(b.constant, money("0.00"));
        assert_eq!(b.threshold, money("0"));
    }

    // 82. A = 58523 → bracket 2 (threshold inclusive at-or-above);
    // A = 58522.99 still bracket 1.
    #[test]
    fn bracket_threshold_inclusive_at_or_above() {
        let at = select_federal_bracket(money("58523.00"), &federal_brackets()).unwrap();
        assert_eq!(at.rate, rate("0.2050"));
        assert_eq!(at.constant, money("3804.00"));

        let just_below = select_federal_bracket(money("58522.99"), &federal_brackets()).unwrap();
        assert_eq!(just_below.rate, rate("0.1400"));
        assert_eq!(just_below.constant, money("0.00"));
    }

    /// 83. A = 300000 → bracket 5, R = 0.3300, K = 26024
    #[test]
    fn bracket_top_rate() {
        let b = select_federal_bracket(money("300000.00"), &federal_brackets()).unwrap();
        assert_eq!(b.rate, rate("0.3300"));
        assert_eq!(b.constant, money("26024.00"));
    }

    // 84. Published-K residual canary: (R×A)−K_published − slices equals
    // K_exact−K_published for A in each bracket, and |residual| ≤ 0.50.
    //
    // The identity holds exactly against K_exact. CRA publishes K rounded to
    // the whole dollar, so the engine's annual tax differs from the true
    // marginal computation by at most fifty cents per bracket, by design of
    // the published table. Pinning the difference to the known residual still
    // catches a transcription error — a mistyped K moves the residual off its
    // known value immediately — while allowing the intended imprecision.
    #[test]
    fn ra_minus_k_minus_slices_equals_k_rounding_residual() {
        let set = crate::rules::loader::load_ruleset_2026_01_01().unwrap();
        for (code, jurisdiction) in &set.jurisdictions {
            let brackets = jurisdiction
                .brackets
                .get(crate::rules::schema::CalculationOption::Option1);
            assert_published_k_residual_identity(&code.0, brackets);
        }
        // Worked example: A = 150000, K_published = 10241, K_exact = 10241.47.
        let fed = federal_brackets();
        let a = money("150000.00");
        let via_k = crate::formulas::k_identity::ra_minus_published_k(a, &fed).unwrap();
        let slices = crate::formulas::k_identity::marginal_slice_sum(a, &fed).unwrap();
        assert_eq!(
            round_tax_to_cent(via_k.checked_div(money("1")).unwrap()),
            money("28759.00")
        );
        assert_eq!(
            round_tax_to_cent(slices.checked_div(money("1")).unwrap()),
            money("28758.53")
        );
        assert_eq!(via_k.checked_sub(slices).unwrap(), money("0.47"));
    }

    fn assert_published_k_residual_identity(code: &str, brackets: &[Bracket]) {
        use crate::formulas::k_identity::{
            k_exact_constants, k_residual, marginal_slice_sum, ra_minus_published_k,
        };
        let exact = k_exact_constants(brackets).unwrap();
        assert_eq!(exact.len(), brackets.len());
        for i in 0..brackets.len() {
            let residual = k_residual(exact[i], brackets[i].constant).unwrap();
            assert!(
                residual <= money("0.50") && residual >= money("-0.50"),
                "{code} bracket {i}: |K_exact − K_published| = {residual} exceeds 0.50"
            );
            let mut samples = vec![brackets[i].threshold];
            if i + 1 < brackets.len() {
                samples.push(
                    brackets[i + 1]
                        .threshold
                        .checked_sub(money("0.01"))
                        .unwrap(),
                );
            } else {
                samples.push(
                    brackets[i]
                        .threshold
                        .checked_add(money("50000.00"))
                        .unwrap(),
                );
            }
            let next = if i + 1 < brackets.len() {
                Some(brackets[i + 1].threshold)
            } else {
                None
            };
            for a in samples {
                if a < brackets[i].threshold {
                    continue;
                }
                if let Some(nxt) = next {
                    if a >= nxt {
                        continue;
                    }
                }
                let via_k = ra_minus_published_k(a, brackets).unwrap();
                let slices = marginal_slice_sum(a, brackets).unwrap();
                let diff = via_k.checked_sub(slices).unwrap();
                assert_eq!(
                    diff, residual,
                    "{code} bracket {i} A={a}: (R×A)−K_pub − slices ({diff}) != K_exact−K_pub ({residual})"
                );
            }
        }
    }

    /// 85. T3 floors at 0 when credits exceed the tax.
    #[test]
    fn t3_floors_at_zero_when_credits_exceed_tax() {
        let t3 = federal_t3(
            money("1000.00"),
            money("500.00"),
            money("500.00"),
            money("0.00"),
            money("200.00"),
            &federal_brackets(),
        )
        .unwrap();
        assert_eq!(t3, money("0.00"));
        assert!(!t3.is_negative());
    }

    /// 86. LCF = lesser of 750 and 15% of the per-period purchase; T1 subtracts P×LCF.
    #[test]
    fn lcf_lesser_of_750_and_fifteen_percent_times_p() {
        let p = PayPeriod::new(26).unwrap();
        let t3 = money("10000.00");
        // LCF = min(750, 0.15×100) = 15 → P×LCF = 390.
        let t1_small = federal_t1(
            t3,
            p,
            Some(money("100.00")),
            &lcf_params(),
            Province::On,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t1_small, money("9610.00"));

        // LCF = min(750, 0.15×10000) = 750 → P×LCF = 19500 → T1 floors at 0.
        let t1_large = federal_t1(
            t3,
            p,
            Some(money("10000.00")),
            &lcf_params(),
            Province::On,
            None,
            None,
        )
        .unwrap();
        assert_eq!(t1_large, money("0.00"));
    }

    /// 87. Quebec abatement: 16.5% of T3 removed, rate from the rule set.
    #[test]
    fn quebec_abatement_uses_ruleset_rate() {
        let t3 = money("10000.00");
        let t1 = federal_t1(
            t3,
            PayPeriod::new(26).unwrap(),
            None,
            &lcf_params(),
            Province::Qc,
            Some(rate("0.165")),
            None,
        )
        .unwrap();
        assert_eq!(t1, money("8350.00"));
        // Different ruleset rate → different T1 (no hard-coded 0.165 in the formula).
        let t1_alt = federal_t1(
            t3,
            PayPeriod::new(26).unwrap(),
            None,
            &lcf_params(),
            Province::Qc,
            Some(rate("0.160")),
            None,
        )
        .unwrap();
        assert_eq!(t1_alt, money("8400.00"));
        assert_ne!(t1, t1_alt);
    }

    /// 88. Outside Canada: 48% surtax on T3; T2 forced to 0 downstream.
    #[test]
    fn outside_canada_surtax_and_t2_zero_downstream() {
        let t3 = money("10000.00");
        let t1 = federal_t1(
            t3,
            PayPeriod::new(26).unwrap(),
            None,
            &lcf_params(),
            Province::OutsideCanada,
            None,
            Some(rate("0.480")),
        )
        .unwrap();
        assert_eq!(t1, money("14800.00"));
        assert_eq!(
            provincial_t2_or_zero(Province::OutsideCanada, money("999.99")),
            money("0.00")
        );
    }

    /// Engine path uses (R×A)−K, not a marginal-slice sum.
    #[test]
    fn t3_applies_top_rate_to_all_of_a() {
        let t3 = federal_t3(
            money("150000.00"),
            money("0.00"),
            money("0.00"),
            money("0.00"),
            money("0.00"),
            &federal_brackets(),
        )
        .unwrap();
        assert_eq!(t3, money("28759.00"));
    }

    // 89. Property: T3 is non-decreasing except for the published-K
    // discontinuity (at most $1.00 drop when A crosses a threshold).
    proptest! {
        #[test]
        fn t3_monotone_in_annual_taxable_income_within_k_rounding_bound(
            a_cents in 0u64..=40_000_000u64,
            b_cents in 0u64..=40_000_000u64,
        ) {
            prop_assume!(a_cents <= b_cents);
            let brackets = federal_brackets();
            let t = |cents: u64| {
                let a = money(&format!("{}.{:02}", cents / 100, cents % 100));
                federal_t3(
                    a,
                    money("0.00"),
                    money("0.00"),
                    money("0.00"),
                    money("0.00"),
                    &brackets,
                )
                .unwrap()
            };
            let lo = t(a_cents);
            let hi = t(b_cents);
            if hi >= lo {
                prop_assert!(true);
            } else {
                prop_assert!(
                    lo.checked_sub(hi).unwrap() <= money("1.00"),
                    "T3 dropped more than the $1.00 K-rounding bound: {lo} → {hi}"
                );
            }
        }
    }

    // 90. Crossing a bracket threshold by one cent changes T3 by at most $1.00,
    // not at most one cent. The discontinuity at threshold i equals
    // (K_exact,i − K_published,i) − (K_exact,i−1 − K_published,i−1);
    // each residual is bounded by 0.50. Measured jumps for every bracket
    // table (FED + twelve provinces/territories, both rule-set versions)
    // live in tests/vectors/bracket_discontinuity_2026.json.
    #[test]
    fn crossing_threshold_by_one_cent_matches_k_rounding_discontinuity() {
        let measured = measure_all_bracket_jumps();
        assert_eq!(
            measured
                .iter()
                .map(|row| row.jurisdiction.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["AB", "BC", "FED", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "SK", "YT"]
                .into_iter()
                .collect(),
            "M-001 table must cover all thirteen bracket jurisdictions"
        );
        for row in &measured {
            let abs_jump = if row.jump.is_negative() {
                -row.jump
            } else {
                row.jump
            };
            assert!(
                abs_jump <= money("1.00"),
                "{} {} {} {}: |jump| {abs_jump} exceeds $1.00 ({} → {})",
                row.rule_set_version,
                row.jurisdiction,
                row.calculation_option,
                row.threshold,
                row.t3_below,
                row.t3_at
            );
            assert_eq!(
                row.same_a_gap,
                row.k_residual_delta,
                "{} {} {} {}: same-A formula gap {} != residual expression {}",
                row.rule_set_version,
                row.jurisdiction,
                row.calculation_option,
                row.threshold,
                row.same_a_gap,
                row.k_residual_delta
            );
        }
        assert_measured_jumps_match_fixture(&measured);
    }

    #[test]
    #[ignore = "set DUMP_M001=1 and run with --ignored --nocapture to regenerate the fixture"]
    fn dump_m001_fixture_when_requested() {
        if std::env::var("DUMP_M001").is_err() {
            return;
        }
        let measured = measure_all_bracket_jumps();
        let mut out = String::from("{\n");
        out.push_str("  \"source_document\": \"T4127 Payroll Deductions Formulas — Table 8.1 / Chapter 4 provincial brackets\",\n");
        out.push_str("  \"note\": \"Jump = T3(threshold) − T3(threshold − 0.01) with published whole-dollar K and zero credits. Negative means annual tax falls when income crosses the threshold. k_residual_delta is (K_exact,i − K_published,i) − (K_exact,i−1 − K_published,i−1) and equals the unrounded same-A formula gap. FED plus twelve provincial tables; Outside Canada has no provincial K.\",\n");
        out.push_str("  \"thresholds\": [\n");
        for (i, row) in measured.iter().enumerate() {
            let delta = row.k_residual_delta.as_decimal().normalize().to_string();
            out.push_str("    {\n");
            out.push_str(&format!(
                "      \"rule_set_version\": \"{}\",\n",
                row.rule_set_version
            ));
            out.push_str(&format!(
                "      \"jurisdiction\": \"{}\",\n",
                row.jurisdiction
            ));
            out.push_str(&format!(
                "      \"calculation_option\": \"{}\",\n",
                row.calculation_option
            ));
            out.push_str(&format!("      \"threshold\": \"{}\",\n", row.threshold));
            out.push_str(&format!("      \"t3_below\": \"{}\",\n", row.t3_below));
            out.push_str(&format!("      \"t3_at\": \"{}\",\n", row.t3_at));
            out.push_str(&format!("      \"jump\": \"{}\",\n", row.jump));
            out.push_str(&format!("      \"k_residual_delta\": \"{delta}\"\n"));
            if i + 1 == measured.len() {
                out.push_str("    }\n");
            } else {
                out.push_str("    },\n");
            }
        }
        out.push_str("  ]\n}\n");
        eprint!("{out}");
    }

    struct MeasuredJump {
        rule_set_version: String,
        jurisdiction: String,
        calculation_option: String,
        threshold: Money,
        t3_below: Money,
        t3_at: Money,
        jump: Money,
        k_residual_delta: Money,
        same_a_gap: Money,
    }

    fn measure_all_bracket_jumps() -> Vec<MeasuredJump> {
        use crate::rules::schema::{CalculationOption, OptionScoped};
        let mut measured = Vec::new();
        for set in [
            crate::rules::loader::load_ruleset_2026_01_01().unwrap(),
            crate::rules::loader::load_ruleset_2026_07_01().unwrap(),
        ] {
            assert_eq!(
                set.jurisdictions.len(),
                13,
                "{} must embed FED plus twelve provincial tables",
                set.rule_set_version
            );
            for (code, jurisdiction) in &set.jurisdictions {
                let options: &[CalculationOption] = match &jurisdiction.brackets {
                    OptionScoped::Both(_) => &[CalculationOption::Option1],
                    OptionScoped::PerOption { .. } => {
                        &[CalculationOption::Option1, CalculationOption::Option2]
                    }
                };
                for option in options {
                    let brackets = jurisdiction.brackets.get(*option);
                    let option_name = match option {
                        CalculationOption::Option1 => "option1",
                        CalculationOption::Option2 => "option2",
                    };
                    for i in 1..brackets.len() {
                        let threshold = brackets[i].threshold;
                        let just_below = threshold.checked_sub(money("0.01")).unwrap();
                        let t = |a: Money| {
                            federal_t3(
                                a,
                                money("0.00"),
                                money("0.00"),
                                money("0.00"),
                                money("0.00"),
                                brackets,
                            )
                            .unwrap()
                        };
                        let t3_below = t(just_below);
                        let t3_at = t(threshold);
                        let jump = t3_at.checked_sub(t3_below).unwrap();
                        let disc =
                            crate::formulas::k_identity::k_rounding_discontinuity(brackets, i)
                                .unwrap();
                        let at_new = (threshold * brackets[i].rate)
                            .checked_sub(brackets[i].constant)
                            .unwrap();
                        let at_old = (threshold * brackets[i - 1].rate)
                            .checked_sub(brackets[i - 1].constant)
                            .unwrap();
                        let same_a_gap = at_new.checked_sub(at_old).unwrap();
                        measured.push(MeasuredJump {
                            rule_set_version: set.rule_set_version.clone(),
                            jurisdiction: code.0.clone(),
                            calculation_option: option_name.to_string(),
                            threshold,
                            t3_below,
                            t3_at,
                            jump,
                            k_residual_delta: disc,
                            same_a_gap,
                        });
                    }
                }
            }
        }
        measured
    }

    fn assert_measured_jumps_match_fixture(measured: &[MeasuredJump]) {
        #[derive(serde::Deserialize)]
        struct Fixture {
            thresholds: Vec<Row>,
        }
        #[derive(serde::Deserialize)]
        struct Row {
            rule_set_version: String,
            jurisdiction: String,
            calculation_option: String,
            threshold: String,
            t3_below: String,
            t3_at: String,
            jump: String,
            k_residual_delta: String,
        }
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/vectors/bracket_discontinuity_2026.json"
        ));
        let fixture: Fixture = serde_json::from_str(raw).expect("k-jump fixture parses");
        assert_eq!(
            fixture.thresholds.len(),
            measured.len(),
            "fixture must list every threshold of all thirteen bracket tables, both editions"
        );
        for (row, got) in fixture.thresholds.iter().zip(measured.iter()) {
            assert_eq!(row.rule_set_version, got.rule_set_version);
            assert_eq!(row.jurisdiction, got.jurisdiction);
            assert_eq!(row.calculation_option, got.calculation_option);
            assert_eq!(money(&row.threshold), got.threshold);
            assert_eq!(money(&row.t3_below), got.t3_below);
            assert_eq!(money(&row.t3_at), got.t3_at);
            assert_eq!(money(&row.jump), got.jump);
            assert_eq!(
                crate::decimal::Money::parse(&row.k_residual_delta).unwrap(),
                got.k_residual_delta,
                "{} {} {} {}: fixture k_residual_delta",
                got.rule_set_version,
                got.jurisdiction,
                got.calculation_option,
                got.threshold
            );
        }
    }

    // 90b. Per-period effect: annual jump / P. At P=52 the largest possible
    // jump (the $1.00 bound) is under two cents of withholding.
    #[test]
    fn per_period_k_jump_at_weekly_is_under_two_cents() {
        let set = crate::rules::loader::load_ruleset_2026_01_01().unwrap();
        let p = money("52");
        let two_cents = crate::decimal::Ratio::parse("0.02").unwrap();
        let bound = money("1.00").checked_div(p).unwrap();
        assert!(
            bound.as_decimal() < two_cents.as_decimal(),
            "largest possible annual jump $1.00 / 52 must be under two cents, got {bound}"
        );
        let mut max_abs = Money::ZERO;
        for jurisdiction in set.jurisdictions.values() {
            let brackets = jurisdiction
                .brackets
                .get(crate::rules::schema::CalculationOption::Option1);
            for i in 1..brackets.len() {
                let threshold = brackets[i].threshold;
                let just_below = threshold.checked_sub(money("0.01")).unwrap();
                let t = |a: Money| {
                    federal_t3(
                        a,
                        money("0.00"),
                        money("0.00"),
                        money("0.00"),
                        money("0.00"),
                        brackets,
                    )
                    .unwrap()
                };
                let jump = t(threshold).checked_sub(t(just_below)).unwrap();
                let abs_jump = if jump.is_negative() { -jump } else { jump };
                if abs_jump > max_abs {
                    max_abs = abs_jump;
                }
            }
        }
        let weekly = max_abs.checked_div(p).unwrap();
        assert!(
            weekly.as_decimal() < two_cents.as_decimal(),
            "measured max annual jump {max_abs} / 52 = {weekly} is not under two cents"
        );
    }
}
