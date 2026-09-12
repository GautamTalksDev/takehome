//! Ontario V1 / V2 / S / Y and provincial assembly (spec §6.4).
//!
//! Three traps in one file:
//! - §21.6 V1 tier two is cumulative (both 20% and 36% terms apply).
//! - §21.7 claim code E still pays V2 (whole-response; deferred to step 11).
//! - V2 is not reduced by S.

use crate::decimal::{DecimalError, Money};
use crate::request::{PayPeriod, Province};
use crate::rounding::round_tax_to_cent;
use crate::rules::schema::{Bracket, HealthPremiumTier, LabourCredit, SurtaxTier, TaxReduction};
use thiserror::Error;

/// Ontario formula failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OntarioTaxError {
    #[error("Ontario tax bracket table is empty or has no bracket for A")]
    MissingBracket,
    #[error("Ontario health-premium table is empty")]
    MissingHealthTier,
    #[error(transparent)]
    Decimal(#[from] DecimalError),
}

fn round_money(value: Money) -> Result<Money, OntarioTaxError> {
    Ok(round_tax_to_cent(value.checked_div(Money::parse("1")?)?))
}

/// Select the highest Ontario bracket whose threshold is at or below A.
pub fn select_ontario_bracket(a: Money, brackets: &[Bracket]) -> Result<Bracket, OntarioTaxError> {
    brackets
        .iter()
        .rev()
        .find(|bracket| a >= bracket.threshold)
        .cloned()
        .ok_or(OntarioTaxError::MissingBracket)
}

/// Ontario tax before surtax, health premium, and reduction.
pub fn ontario_t4(
    a: Money,
    brackets: &[Bracket],
    k1p: Money,
    k2p: Money,
    k3p: Money,
    k4p: Money,
    k5p: Money,
) -> Result<Money, OntarioTaxError> {
    let bracket = select_ontario_bracket(a, brackets)?;
    round_money(
        a.checked_mul_rate(bracket.rate)?
            .checked_sub(bracket.constant)?
            .checked_sub(k1p)?
            .checked_sub(k2p)?
            .checked_sub(k3p)?
            .checked_sub(k4p)?
            .checked_sub(k5p)?
            .floor_at_zero(),
    )
}

/// Ontario surtax V1. Every occupied tier contributes cumulatively.
pub fn ontario_v1(t4: Money, tiers: &[SurtaxTier]) -> Result<Money, OntarioTaxError> {
    let mut total = Money::ZERO;
    for tier in tiers {
        if t4 > tier.threshold {
            total = total.checked_add(
                t4.checked_sub(tier.threshold)?
                    .checked_mul_rate(tier.rate)?,
            )?;
        }
    }
    round_money(total)
}

/// Ontario health premium V2, using exclusive lower tier thresholds.
pub fn ontario_v2(a: Money, tiers: &[HealthPremiumTier]) -> Result<Money, OntarioTaxError> {
    let tier = tiers
        .iter()
        .rev()
        .find(|tier| a > tier.threshold || tier.threshold.is_zero())
        .ok_or(OntarioTaxError::MissingHealthTier)?;
    let excess = a.checked_sub(tier.threshold)?.floor_at_zero();
    round_money(
        tier.base
            .checked_add(excess.checked_mul_rate(tier.rate)?)?
            .min(tier.cap),
    )
}

/// Ontario dependant amount Y.
pub fn ontario_y(
    dependants_disabled: u8,
    dependants_under_19: u8,
    per_dependant: Money,
) -> Result<Money, OntarioTaxError> {
    let count = u16::from(dependants_disabled) + u16::from(dependants_under_19);
    per_dependant
        .checked_mul(Money::parse(&count.to_string())?)
        .map_err(OntarioTaxError::from)
}

/// Ontario tax reduction S.
pub fn ontario_s(
    t4: Money,
    v1: Money,
    y: Money,
    reduction: &TaxReduction,
) -> Result<Money, OntarioTaxError> {
    let tax = t4.checked_add(v1)?;
    let two = Money::parse("2")?;
    let available = reduction
        .basic
        .checked_add(y)?
        .checked_mul(two)?
        .checked_sub(tax)?
        .floor_at_zero();
    round_money(tax.min(available))
}

fn labour_credit(
    purchase: Option<Money>,
    params: Option<&LabourCredit>,
) -> Result<Money, OntarioTaxError> {
    match (purchase, params) {
        (Some(purchase), Some(params)) => round_money(
            purchase
                .floor_at_zero()
                .checked_mul_rate(params.rate)?
                .min(params.max),
        ),
        _ => Ok(Money::ZERO),
    }
}

/// T2 = max(0, T4 + V1 + V2 − S − P×LCP). V2 is never reduced separately.
#[allow(clippy::too_many_arguments)]
pub fn ontario_t2(
    t4: Money,
    v1: Money,
    v2: Money,
    s: Money,
    pay_periods: PayPeriod,
    lcp_purchase: Option<Money>,
    lcp_params: Option<&LabourCredit>,
) -> Result<Money, OntarioTaxError> {
    let p = Money::parse(&pay_periods.get().to_string())?;
    let lcp = labour_credit(lcp_purchase, lcp_params)?;
    round_money(
        t4.checked_add(v1)?
            .checked_add(v2)?
            .checked_sub(s)?
            .checked_sub(lcp.checked_mul(p)?)?
            .floor_at_zero(),
    )
}

/// Provincial T2 dispatch; Quebec and Outside Canada are always zero.
#[allow(clippy::too_many_arguments)]
pub fn provincial_t2(
    province: Province,
    t4: Money,
    v1: Money,
    v2: Money,
    s: Money,
    pay_periods: PayPeriod,
    lcp_purchase: Option<Money>,
    lcp_params: Option<&LabourCredit>,
) -> Result<Money, OntarioTaxError> {
    match province {
        Province::Qc | Province::OutsideCanada => Ok(Money::ZERO),
        _ => ontario_t2(t4, v1, v2, s, pay_periods, lcp_purchase, lcp_params),
    }
}

#[cfg(test)]
mod tests {
    use crate::decimal::{Money, Rate};
    use crate::request::{PayPeriod, Province};
    use crate::rounding::round_tax_to_cent;
    use crate::rules::schema::{
        Bracket, HealthPremiumTier, LabourCredit, SurtaxTier, TaxReduction,
    };
    use proptest::prelude::*;

    use super::{
        ontario_s, ontario_t2, ontario_t4, ontario_v1, ontario_v2, ontario_y, provincial_t2,
        select_ontario_bracket,
    };

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }
    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    /// 2026 Ontario surtax thresholds (on.json / T4127).
    fn surtax_tiers() -> Vec<SurtaxTier> {
        vec![
            SurtaxTier {
                threshold: money("5818.00"),
                rate: rate("0.20"),
            },
            SurtaxTier {
                threshold: money("7446.00"),
                rate: rate("0.36"),
            },
        ]
    }

    /// 2026 Ontario Health Premium tiers (on.json). Threshold is exclusive lower bound.
    fn health_tiers() -> Vec<HealthPremiumTier> {
        vec![
            HealthPremiumTier {
                threshold: money("0.00"),
                rate: rate("0.00"),
                base: money("0.00"),
                cap: money("0.00"),
            },
            HealthPremiumTier {
                threshold: money("20000.00"),
                rate: rate("0.06"),
                base: money("0.00"),
                cap: money("300.00"),
            },
            HealthPremiumTier {
                threshold: money("36000.00"),
                rate: rate("0.06"),
                base: money("300.00"),
                cap: money("450.00"),
            },
            HealthPremiumTier {
                threshold: money("48000.00"),
                rate: rate("0.25"),
                base: money("450.00"),
                cap: money("600.00"),
            },
            HealthPremiumTier {
                threshold: money("72000.00"),
                rate: rate("0.25"),
                base: money("600.00"),
                cap: money("750.00"),
            },
            HealthPremiumTier {
                threshold: money("200000.00"),
                rate: rate("0.25"),
                base: money("750.00"),
                cap: money("900.00"),
            },
        ]
    }

    fn ontario_brackets() -> Vec<Bracket> {
        vec![
            Bracket {
                threshold: money("0"),
                rate: rate("0.0505"),
                constant: money("0.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("53891.00"),
                rate: rate("0.0915"),
                constant: money("2210.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("107785.00"),
                rate: rate("0.1116"),
                constant: money("4376.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("150000.00"),
                rate: rate("0.1216"),
                constant: money("5876.00"),
                prorated: false,
            },
            Bracket {
                threshold: money("220000.00"),
                rate: rate("0.1316"),
                constant: money("8076.00"),
                prorated: false,
            },
        ]
    }

    fn tax_reduction() -> TaxReduction {
        TaxReduction {
            basic: money("300.00"),
            dependant: money("554.00"),
        }
    }

    /// Per-dependant Y amount (T4127 Ontario / Form TD1ON), 2026.
    fn y_per_dependant() -> Money {
        money("554.00")
    }

    // --- SURTAX V1 -----------------------------------------------------------

    /// 91. T4 = 5000 → 0.00
    #[test]
    fn v1_below_first_threshold_is_zero() {
        assert_eq!(
            ontario_v1(money("5000.00"), &surtax_tiers()).unwrap(),
            money("0.00")
        );
    }

    /// 92. T4 = 5818 → 0.00 (boundary inclusive → still zero)
    #[test]
    fn v1_at_first_threshold_is_zero() {
        assert_eq!(
            ontario_v1(money("5818.00"), &surtax_tiers()).unwrap(),
            money("0.00")
        );
    }

    /// 93. T4 = 7000 → 0.20 × 1182 = 236.40
    #[test]
    fn v1_mid_first_tier() {
        assert_eq!(
            ontario_v1(money("7000.00"), &surtax_tiers()).unwrap(),
            money("236.40")
        );
    }

    /// 94. T4 = 7446 → 0.20 × 1628 = 325.60
    #[test]
    fn v1_at_second_threshold_still_first_tier_only() {
        assert_eq!(
            ontario_v1(money("7446.00"), &surtax_tiers()).unwrap(),
            money("325.60")
        );
    }

    /// 95. T4 = 10000 → BOTH terms apply (cumulative). Non-cumulative = 919.44 trap.
    #[test]
    fn v1_second_tier_is_cumulative() {
        // 0.20 × (10000 − 5818) + 0.36 × (10000 − 7446)
        // = 0.20 × 4182 + 0.36 × 2554
        // = 836.40 + 919.44 = 1755.84
        let v1 = ontario_v1(money("10000.00"), &surtax_tiers()).unwrap();
        assert_eq!(v1, money("1755.84"));
        assert_ne!(
            v1,
            money("919.44"),
            "trap §21.6: non-cumulative second tier alone looks plausible"
        );
    }

    // 96. Property: V1 is continuous at both boundaries (no jump).
    proptest! {
        #[test]
        fn v1_continuous_at_surtax_boundaries(which in 0usize..=1usize) {
            let tiers = surtax_tiers();
            let thr = tiers[which].threshold;
            let just_below = thr.checked_sub(money("0.01")).unwrap();
            let lo = ontario_v1(just_below, &tiers).unwrap();
            let hi = ontario_v1(thr, &tiers).unwrap();
            let delta = if hi >= lo {
                hi.checked_sub(lo).unwrap()
            } else {
                lo.checked_sub(hi).unwrap()
            };
            prop_assert!(
                delta <= money("0.01"),
                "V1 jump at {}: {} → {} delta {}",
                thr,
                lo,
                hi,
                delta
            );
        }
    }

    // --- HEALTH PREMIUM V2 ---------------------------------------------------

    /// 97. Table-driven: one interior + one exact boundary per six tiers (≥12 cases).
    #[test]
    fn v2_table_driven_six_tiers_interiors_and_boundaries() {
        let tiers = health_tiers();
        let cases: &[(&str, &str)] = &[
            // Boundaries (at each tier threshold).
            ("0.00", "0.00"),
            ("20000.00", "0.00"),
            ("36000.00", "300.00"),
            ("48000.00", "450.00"),
            ("72000.00", "600.00"),
            ("200000.00", "750.00"),
            // Strictly inside each tier (not sitting on a cap when avoidable).
            ("10000.00", "0.00"),
            ("22000.00", "120.00"),
            ("37000.00", "360.00"),
            ("48200.00", "500.00"),
            ("72400.00", "700.00"),
            ("200400.00", "850.00"),
        ];
        assert!(cases.len() >= 12);
        for &(a, expected) in cases {
            assert_eq!(
                ontario_v2(money(a), &tiers).unwrap(),
                money(expected),
                "V2 at A={a}"
            );
        }
    }

    /// 98. A = 20000 → 0.00; A = 20001 → 0.06 × 1 = 0.06
    #[test]
    fn v2_crosses_20000_by_one_dollar() {
        let tiers = health_tiers();
        assert_eq!(
            ontario_v2(money("20000.00"), &tiers).unwrap(),
            money("0.00")
        );
        assert_eq!(
            ontario_v2(money("20001.00"), &tiers).unwrap(),
            money("0.06")
        );
    }

    /// 99. A = 200001 → 750 + 0.25 × 1 = 750.25, under the 900 cap
    #[test]
    fn v2_just_into_top_tier() {
        assert_eq!(
            ontario_v2(money("200001.00"), &health_tiers()).unwrap(),
            money("750.25")
        );
    }

    /// 100. A = 1000000 → 900.00 (the cap)
    #[test]
    fn v2_hits_top_cap() {
        assert_eq!(
            ontario_v2(money("1000000.00"), &health_tiers()).unwrap(),
            money("900.00")
        );
    }

    // 101. SPEC §21.7 — claim code E: V2 still payable while T4 = 0.
    // Covered by `lib.rs::claim_code_e_still_pays_ontario_health_premium`
    // (whole-response). Empty stub deleted per M1 rule: #[ignore] is never a
    // resolution.

    /// 102. V2 is NOT reduced by S (T4127 note). Both non-zero; T2 still includes full V2.
    #[test]
    fn v2_is_not_reduced_by_s() {
        let t4 = money("5975.00");
        let v1 = ontario_v1(t4, &surtax_tiers()).unwrap();
        let v2 = ontario_v2(money("90000.00"), &health_tiers()).unwrap();
        let y = ontario_y(5, 0, y_per_dependant()).unwrap();
        let s = ontario_s(t4, v1, y, &tax_reduction()).unwrap();
        assert!(v2 > money("0.00"), "need non-zero V2");
        assert!(s > money("0.00"), "need non-zero S");

        let t2 = ontario_t2(t4, v1, v2, s, PayPeriod::new(26).unwrap(), None, None).unwrap();

        // If S wrongly ate into V2, T2 would be lower than T4+V1+V2−S.
        let expected = t4
            .checked_add(v1)
            .unwrap()
            .checked_add(v2)
            .unwrap()
            .checked_sub(s)
            .unwrap();
        assert_eq!(t2, expected);
        assert_eq!(v2, money("750.00"));
    }

    // --- TAX REDUCTION S / Y -------------------------------------------------

    /// 103. T4 + V1 = 200, Y = 0 → S = lesser of 200 and (600 − 200) = 200
    #[test]
    fn s_fully_covers_small_tax() {
        assert_eq!(
            ontario_s(
                money("200.00"),
                money("0.00"),
                money("0.00"),
                &tax_reduction()
            )
            .unwrap(),
            money("200.00")
        );
    }

    /// 104. T4 + V1 = 500, Y = 0 → S = lesser of 500 and 100 = 100
    #[test]
    fn s_partial_phase_out() {
        assert_eq!(
            ontario_s(
                money("500.00"),
                money("0.00"),
                money("0.00"),
                &tax_reduction()
            )
            .unwrap(),
            money("100.00")
        );
    }

    /// 105. T4 + V1 = 700, Y = 0 → S = 0 (floored; 600 − 700 < 0)
    #[test]
    fn s_floors_at_zero_past_phase_out() {
        assert_eq!(
            ontario_s(
                money("700.00"),
                money("0.00"),
                money("0.00"),
                &tax_reduction()
            )
            .unwrap(),
            money("0.00")
        );
    }

    /// 106. Y = 1108 (two dependants) shifts the phase-out.
    #[test]
    fn y_two_dependants_shifts_phase_out() {
        let y = ontario_y(1, 1, y_per_dependant()).unwrap();
        assert_eq!(y, money("1108.00"));
        // 2 × (300 + 1108) − 2000 = 2816 − 2000 = 816; lesser of 2000 and 816.
        assert_eq!(
            ontario_s(money("2000.00"), money("0.00"), y, &tax_reduction()).unwrap(),
            money("816.00")
        );
        // Same tax with Y=0 would already be fully phased out (S=0).
        assert_eq!(
            ontario_s(
                money("2000.00"),
                money("0.00"),
                money("0.00"),
                &tax_reduction()
            )
            .unwrap(),
            money("0.00")
        );
    }

    // 107. Property: S ≤ T4 + V1 always (so T2 never goes negative from S alone).
    proptest! {
        #[test]
        fn s_never_exceeds_t4_plus_v1(
            t4_cents in 0u64..=2_000_000u64,
            v1_cents in 0u64..=500_000u64,
            disabled in 0u8..=10u8,
            under_19 in 0u8..=10u8,
        ) {
            let t4 = money(&format!("{}.{:02}", t4_cents / 100, t4_cents % 100));
            let v1 = money(&format!("{}.{:02}", v1_cents / 100, v1_cents % 100));
            let y = ontario_y(disabled, under_19, y_per_dependant()).unwrap();
            let s = ontario_s(t4, v1, y, &tax_reduction()).unwrap();
            let tv = t4.checked_add(v1).unwrap();
            prop_assert!(s <= tv);
        }
    }

    // --- PROVINCIAL ASSEMBLY -------------------------------------------------

    /// 108. Full Ontario case: every assembly term non-zero, to the cent.
    #[test]
    fn full_ontario_assembly_every_term_nonzero() {
        let a = money("90000.00");
        let brackets = ontario_brackets();
        let b = select_ontario_bracket(a, &brackets).unwrap();
        assert_eq!(b.rate, rate("0.0915"));
        assert_eq!(b.constant, money("2210.00"));

        // Credits split so each K*P term is non-zero (K5P included for the generic T4 formula).
        let t4 = ontario_t4(
            a,
            &brackets,
            money("10.00"),
            money("10.00"),
            money("10.00"),
            money("10.00"),
            money("10.00"),
        )
        .unwrap();
        assert_eq!(t4, money("5975.00"));

        let v1 = ontario_v1(t4, &surtax_tiers()).unwrap();
        assert_eq!(v1, money("31.40"));

        let v2 = ontario_v2(a, &health_tiers()).unwrap();
        assert_eq!(v2, money("750.00"));

        let y = ontario_y(5, 0, y_per_dependant()).unwrap();
        assert_eq!(y, money("2770.00"));
        let s = ontario_s(t4, v1, y, &tax_reduction()).unwrap();
        assert_eq!(s, money("133.60"));

        // LCP not in ON rules; still exercise P×LCP with a labour-credit shape.
        let lcp = LabourCredit {
            rate: rate("0.150"),
            max: money("750.00"),
        };
        let p = PayPeriod::new(26).unwrap();
        let t2 = ontario_t2(t4, v1, v2, s, p, Some(money("50.00")), Some(&lcp)).unwrap();
        // T2 = 5975 + 31.40 + 750 − 133.60 − (26 × min(750, 0.15×50))
        //    = 6756.40 − 133.60 − 195 = 6427.80
        assert_eq!(t2, money("6427.80"));
        assert!(t4 > money("0.00"));
        assert!(v1 > money("0.00"));
        assert!(v2 > money("0.00"));
        assert!(s > money("0.00"));
        assert!(money("195.00") > money("0.00")); // P×LCP
    }

    /// 109. Quebec → T2 = 0 always
    #[test]
    fn quebec_t2_is_always_zero() {
        assert_eq!(
            provincial_t2(
                Province::Qc,
                money("9999.00"),
                money("100.00"),
                money("750.00"),
                money("50.00"),
                PayPeriod::new(26).unwrap(),
                None,
                None,
            )
            .unwrap(),
            money("0.00")
        );
    }

    /// 110. Outside Canada → T2 = 0 always
    #[test]
    fn outside_canada_t2_is_always_zero() {
        assert_eq!(
            provincial_t2(
                Province::OutsideCanada,
                money("9999.00"),
                money("100.00"),
                money("750.00"),
                money("50.00"),
                PayPeriod::new(26).unwrap(),
                None,
                None,
            )
            .unwrap(),
            money("0.00")
        );
    }

    // 111. Property: T2 ≥ 0
    proptest! {
        #[test]
        fn t2_never_negative(
            t4_cents in 0u64..=3_000_000u64,
            a_cents in 0u64..=50_000_000u64,
            disabled in 0u8..=6u8,
            under_19 in 0u8..=6u8,
            purchase_cents in 0u64..=200_000u64,
        ) {
            let t4 = money(&format!("{}.{:02}", t4_cents / 100, t4_cents % 100));
            let a = money(&format!("{}.{:02}", a_cents / 100, a_cents % 100));
            let purchase = money(&format!("{}.{:02}", purchase_cents / 100, purchase_cents % 100));
            let v1 = ontario_v1(t4, &surtax_tiers()).unwrap();
            let v2 = ontario_v2(a, &health_tiers()).unwrap();
            let y = ontario_y(disabled, under_19, y_per_dependant()).unwrap();
            let s = ontario_s(t4, v1, y, &tax_reduction()).unwrap();
            let lcp = LabourCredit {
                rate: rate("0.150"),
                max: money("750.00"),
            };
            let t2 = ontario_t2(
                t4,
                v1,
                v2,
                s,
                PayPeriod::new(26).unwrap(),
                Some(purchase),
                Some(&lcp),
            )
            .unwrap();
            prop_assert!(!t2.is_negative());
        }
    }

    /// Engine path applies top occupied V to all of A (KP constant), like federal R/K.
    #[test]
    fn t4_applies_top_rate_to_all_of_a() {
        let a = money("90000.00");
        let b = select_ontario_bracket(a, &ontario_brackets()).unwrap();
        let via_kp = round_tax_to_cent(
            (a * b.rate)
                .checked_sub(b.constant)
                .unwrap()
                .checked_div(money("1"))
                .unwrap(),
        );
        assert_eq!(via_kp, money("6025.00"));
        let t4 = ontario_t4(
            a,
            &ontario_brackets(),
            money("0.00"),
            money("0.00"),
            money("0.00"),
            money("0.00"),
            money("0.00"),
        )
        .unwrap();
        assert_eq!(t4, money("6025.00"));
    }
}
