//! Spec §16.2 invariants across the thirteen employment jurisdictions.
//!
//! Ontario's 10k property remains the M1 canary in `lib.rs`. This module
//! runs the same claims on every supported province/territory/Outside Canada
//! and every embedded rule-set version.

use super::{calculate, ClaimCode, EngineError, Money, PayPeriod, Province, Request, Response};
use crate::decimal::Rate;
use crate::formulas::k_identity::k_rounding_discontinuity;
use crate::formulas::per_period::per_period_tax;
use crate::request::K2Method;
use crate::rounding::{round_contribution_to_cent, round_tax_to_cent, Granularity};
use crate::rules::loader::{load_ruleset_2026_01_01, load_ruleset_2026_07_01, EMBEDDED_REGISTRY};
use crate::rules::schema::{
    CalculationOption, CalendarDate, JurisdictionCode, OptionScoped, RuleSet,
};
use proptest::prelude::*;
use std::str::FromStr;

const EMPLOYMENT: &[Province] = &[
    Province::Ab,
    Province::Bc,
    Province::Mb,
    Province::Nb,
    Province::Nl,
    Province::Ns,
    Province::Nt,
    Province::Nu,
    Province::On,
    Province::Pe,
    Province::Sk,
    Province::Yt,
    Province::OutsideCanada,
];

const RULE_SET_DATES: &[&str] = &["2026-01-01", "2026-07-01", "2027-01-01"];

fn money(s: &str) -> Money {
    Money::parse(s).unwrap()
}

fn date(s: &str) -> CalendarDate {
    CalendarDate::from_str(s).unwrap()
}

fn cents_money(cents: u64) -> Money {
    money(&format!("{}.{:02}", cents / 100, cents % 100))
}

fn blank_request(
    province: Province,
    as_of: &str,
    periods: u16,
    gross: Money,
    claim: u8,
) -> Request {
    Request {
        as_of: date(as_of),
        province,
        pay_period: PayPeriod::new(periods).unwrap(),
        gross_pay: gross,
        calculation_option: CalculationOption::Option1,
        pensionable_earnings: None,
        insurable_earnings: None,
        federal_claim_code: Some(ClaimCode::Code(claim)),
        provincial_claim_code: Some(ClaimCode::Code(claim)),
        federal_tc: None,
        provincial_tcp: None,
        cpp_months: 12,
        k2_method: K2Method::PdocObserved,
        bonus_method: crate::request::BonusMethod::Regular,
        rounding_compat: crate::request::RoundingCompat::T4127,
        ytd_pensionable_earnings: None,
        ytd_insurable_earnings: None,
        ytd_cpp: None,
        ytd_cpp2: None,
        ytd_ei: None,
        ytd_federal_tax: None,
        ytd_provincial_tax: None,
        pay_periods_elapsed: None,
        bonus: None,
        retroactive_pay: None,
        ytd_bonus: None,
        f5b_ytd: None,
        most_recent_i: None,
        rpp: None,
        bonus_rrsp: None,
        ytd_bonus_rrsp: None,
        ytd_income: None,
        ytd_rpp: None,
        ytd_union_dues: None,
        f5a_ytd: None,
        commission_income: None,
        commission_expenses: None,
        estimated_annual_expenses: None,
        ytd_qpp: None,
        ytd_qpp2: None,
        ytd_qpip: None,
        prior_province: None,
        date_of_birth: None,
        cpp_exempt: None,
        cpp_election_after_65: None,
        additional_tax_requested: None,
        taxable_benefits: None,
        union_dues: None,
        alimony: None,
        child_care_expenses: None,
        prescribed_zone_deduction: None,
        lcf_purchase: None,
        lcp_purchase: None,
        dependants_under_19: None,
        dependants_disabled: None,
    }
}

fn headline_tax(resp: &Response) -> Money {
    resp.employee.total_tax
}

fn max_negative_k_jump(brackets: &[crate::rules::schema::Bracket]) -> Money {
    let mut worst = Money::ZERO;
    for i in 1..brackets.len() {
        let disc = k_rounding_discontinuity(brackets, i).unwrap();
        if disc.is_negative() && disc < worst {
            worst = disc;
        }
    }
    if worst.is_negative() {
        -worst
    } else {
        Money::ZERO
    }
}

fn brackets_for<'a>(set: &'a RuleSet, code: &str) -> &'a [crate::rules::schema::Bracket] {
    set.jurisdictions
        .get(&JurisdictionCode(code.to_string()))
        .unwrap()
        .brackets
        .get(CalculationOption::Option1)
}

fn surtax_top_rate(set: &RuleSet, province: Province) -> Rate {
    if province == Province::OutsideCanada {
        return set
            .jurisdictions
            .get(&JurisdictionCode("FED".to_string()))
            .unwrap()
            .surtax_flat
            .unwrap_or(Rate::parse("0").unwrap());
    }
    let Some(j) = set
        .jurisdictions
        .get(&JurisdictionCode(province.as_str().to_string()))
    else {
        return Rate::parse("0").unwrap();
    };
    j.surtax
        .as_ref()
        .and_then(|tiers| tiers.last())
        .map(|tier| tier.rate)
        .unwrap_or(Rate::parse("0").unwrap())
}

/// Period-tax drop bound from M-001: |K residual difference| per table,
/// provincial surtax amplifying a T4 drop, Outside-Canada 48% on T1,
/// plus two cents of per-period rounding.
fn m001_period_bound(set: &RuleSet, province: Province, periods: u16) -> Money {
    let fed = max_negative_k_jump(brackets_for(set, "FED"));
    let prov = if province == Province::OutsideCanada {
        Money::ZERO
    } else {
        max_negative_k_jump(brackets_for(set, province.as_str()))
    };
    let fed_annual = if province == Province::OutsideCanada {
        fed.checked_add(
            fed.checked_mul_rate(surtax_top_rate(set, province))
                .unwrap(),
        )
        .unwrap()
    } else {
        fed
    };
    let prov_annual = prov
        .checked_add(
            prov.checked_mul_rate(surtax_top_rate(set, province))
                .unwrap(),
        )
        .unwrap();
    let annual = fed_annual.checked_add(prov_annual).unwrap();
    let p = Money::parse(&periods.to_string()).unwrap();
    round_tax_to_cent(annual.checked_div(p).unwrap())
        .checked_add(money("0.02"))
        .unwrap()
}

fn cpp_cap(set: &RuleSet, cpp_months: u8) -> Money {
    round_contribution_to_cent(
        set.cpp
            .total_max
            .checked_mul(Money::parse(&cpp_months.to_string()).unwrap())
            .unwrap()
            .checked_div(money("12"))
            .unwrap(),
    )
}

fn cpp2_cap(set: &RuleSet, cpp_months: u8) -> Money {
    round_contribution_to_cent(
        set.cpp
            .second_additional_max
            .checked_mul(Money::parse(&cpp_months.to_string()).unwrap())
            .unwrap()
            .checked_div(money("12"))
            .unwrap(),
    )
}

fn assert_invariants(req: &Request, resp: &Response, set: &RuleSet) {
    assert!(
        resp.employee.total_deductions <= req.gross_pay,
        "{} {}: deductions {} > gross {}",
        req.province,
        req.as_of,
        resp.employee.total_deductions,
        req.gross_pay
    );
    assert!(!resp.employee.net_pay.is_negative());
    assert_eq!(
        resp.employee.net_pay,
        req.gross_pay
            .checked_sub(resp.employee.total_deductions)
            .unwrap()
    );
    for amount in [
        resp.employee.federal_tax,
        resp.employee.provincial_tax,
        resp.employee.cpp,
        resp.employee.cpp2,
        resp.employee.ei,
        resp.employee.qpip,
    ] {
        assert!(!amount.is_negative());
    }
    assert!(resp.employee.cpp <= cpp_cap(set, req.cpp_months));
    assert!(resp.employee.cpp2 <= cpp2_cap(set, req.cpp_months));
    assert_eq!(
        set.cpp.second_additional_max,
        money("416.00"),
        "CPP2 annual max is $416 while YMPE/YAMPE remain at 2026 published dollars; the invariant is C2 ≤ 416 × PM/12"
    );
    assert!(resp.employee.ei <= set.ei.employee_max);
    let l = req.additional_tax_requested.unwrap_or(Money::ZERO);
    let t = per_period_tax(
        resp.breakdown.t1,
        resp.breakdown.t2,
        req.pay_period,
        l,
        Granularity::Cent,
    )
    .unwrap();
    let federal_period = per_period_tax(
        resp.breakdown.t1,
        Money::ZERO,
        req.pay_period,
        l,
        Granularity::Cent,
    )
    .unwrap();
    let provincial_period = per_period_tax(
        Money::ZERO,
        resp.breakdown.t2,
        req.pay_period,
        Money::ZERO,
        Granularity::Cent,
    )
    .unwrap();
    assert_eq!(resp.breakdown.t, t, "T = [(T1+T2)/P]+L");
    assert_eq!(resp.employee.federal_tax, federal_period);
    assert_eq!(resp.employee.provincial_tax, provincial_period);
    assert_eq!(
        resp.employee.total_tax,
        federal_period.checked_add(provincial_period).unwrap(),
        "total_tax is the sum of the two rounded PDOC lines"
    );
    // round(T1/P)+round(T2/P) can differ from round((T1+T2)/P) by one cent.
    let split = headline_tax(resp);
    let gap = if split >= t {
        split.checked_sub(t).unwrap()
    } else {
        t.checked_sub(split).unwrap()
    };
    assert!(
        gap <= money("0.01"),
        "split period tax {split} vs T {t} differs by {gap}"
    );
}

/// Smoke: every employment jurisdiction × both rule-set versions calculates.
#[test]
fn every_jurisdiction_and_rule_set_calculates() {
    assert_eq!(EMPLOYMENT.len(), 13);
    for province in EMPLOYMENT {
        for as_of in RULE_SET_DATES {
            let req = blank_request(*province, as_of, 52, money("1000.00"), 1);
            let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap_or_else(|err| {
                panic!("{province} {as_of}: {err}");
            });
            let set = EMBEDDED_REGISTRY.resolve(req.as_of).unwrap();
            assert_eq!(resp.rule_set_version, set.rule_set_version);
            assert_invariants(&req, &resp, set);
        }
    }
}

/// Test 41 — same request at 2026-01-01 and 2026-06-30 is identical for all 13.
#[test]
fn january_interval_is_identical_on_jan_1_and_june_30() {
    for province in EMPLOYMENT {
        let jan = calculate(
            &blank_request(*province, "2026-01-01", 52, money("1000.00"), 1),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        let june = calculate(
            &blank_request(*province, "2026-06-30", 52, money("1000.00"), 1),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert_eq!(jan.rule_set_version, "2026-01-01", "{province:?}");
        assert_eq!(june.rule_set_version, "2026-01-01", "{province:?}");
        let a = serde_json::to_string(&jan).unwrap();
        let b = serde_json::to_string(&june).unwrap();
        assert_eq!(a, b, "{province:?} 2026-01-01 vs 2026-06-30");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    /// Test 38 — §16.2 over every jurisdiction and both rule-set versions.
    #[test]
    fn calculate_invariants_all_jurisdictions(
        province in prop::sample::select(EMPLOYMENT.to_vec()),
        as_of in prop::sample::select(RULE_SET_DATES.to_vec()),
        gross_cents in 1u64..=5_000_000u64,
        gross_hi_cents in 1u64..=5_000_000u64,
        periods in prop::sample::select(PayPeriod::LEGAL.to_vec()),
        claim in 0u8..=1u8,
        cpp_months in 1u8..=12u8,
    ) {
        let lo = gross_cents.min(gross_hi_cents);
        let hi = gross_cents.max(gross_hi_cents);
        let mk = |cents: u64| {
            let mut r = blank_request(province, as_of, periods, cents_money(cents), claim);
            r.cpp_months = cpp_months;
            r
        };
        let req_lo = mk(lo);
        let req_hi = mk(hi);
        let resp_lo = calculate(&req_lo, &EMBEDDED_REGISTRY).unwrap();
        let resp_hi = calculate(&req_hi, &EMBEDDED_REGISTRY).unwrap();
        let set = EMBEDDED_REGISTRY.resolve(req_lo.as_of).unwrap();
        assert_invariants(&req_lo, &resp_lo, set);
        assert_invariants(&req_hi, &resp_hi, set);

        let tax_lo = headline_tax(&resp_lo);
        let tax_hi = headline_tax(&resp_hi);
        if tax_lo > tax_hi {
            let drop = tax_lo.checked_sub(tax_hi).unwrap();
            let bound = m001_period_bound(set, province, periods);
            prop_assert!(
                drop <= bound,
                "period tax dropped more than the M-001 bound {bound}: {tax_lo} → {tax_hi} ({} {} P={periods} {}→{})",
                province,
                as_of,
                req_lo.gross_pay,
                req_hi.gross_pay
            );
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]
    /// Increasing TC / TCP / claim code never increases tax.
    #[test]
    fn increasing_credits_never_increases_tax_all_jurisdictions(
        province in prop::sample::select(EMPLOYMENT.to_vec()),
        as_of in prop::sample::select(RULE_SET_DATES.to_vec()),
        gross_cents in 10_000u64..=2_000_000u64,
        tc_lo in 0u64..=1_645_200u64,
        tc_delta in 0u64..=500_000u64,
        claim_lo in 0u8..=10u8,
        claim_hi in 0u8..=10u8,
        periods in prop::sample::select(vec![12u16, 24, 26, 52]),
    ) {
        let lo_claim = claim_lo.min(claim_hi);
        let hi_claim = claim_lo.max(claim_hi);
        let mut a = blank_request(province, as_of, periods, cents_money(gross_cents), lo_claim);
        let mut b = blank_request(province, as_of, periods, cents_money(gross_cents), hi_claim);
        let tax_claim_a = headline_tax(&calculate(&a, &EMBEDDED_REGISTRY).unwrap());
        let tax_claim_b = headline_tax(&calculate(&b, &EMBEDDED_REGISTRY).unwrap());
        prop_assert!(tax_claim_b <= tax_claim_a);

        a.federal_claim_code = None;
        a.provincial_claim_code = None;
        b.federal_claim_code = None;
        b.provincial_claim_code = None;
        let tc_a = cents_money(tc_lo);
        let tc_b = tc_a.checked_add(cents_money(tc_delta)).unwrap();
        a.federal_tc = Some(tc_a);
        a.provincial_tcp = Some(tc_a);
        b.federal_tc = Some(tc_b);
        b.provincial_tcp = Some(tc_b);
        let tax_tc_a = headline_tax(&calculate(&a, &EMBEDDED_REGISTRY).unwrap());
        let tax_tc_b = headline_tax(&calculate(&b, &EMBEDDED_REGISTRY).unwrap());
        prop_assert!(tax_tc_b <= tax_tc_a);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    /// Test 40 — 10k random requests, calculate twice, byte-identical JSON.
    #[test]
    fn calculate_is_byte_identical_across_ten_thousand_requests(
        province in prop::sample::select(EMPLOYMENT.to_vec()),
        as_of in prop::sample::select(RULE_SET_DATES.to_vec()),
        gross_cents in 1u64..=5_000_000u64,
        periods in prop::sample::select(PayPeriod::LEGAL.to_vec()),
        claim in 0u8..=1u8,
        cpp_months in 1u8..=12u8,
    ) {
        let mut req = blank_request(province, as_of, periods, cents_money(gross_cents), claim);
        req.cpp_months = cpp_months;
        let first = serde_json::to_vec(&calculate(&req, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        let second = serde_json::to_vec(&calculate(&req, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        prop_assert_eq!(first, second);
        let req_a = serde_json::to_vec(&req).unwrap();
        let req_b = serde_json::to_vec(&req).unwrap();
        prop_assert_eq!(req_a, req_b);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]
    /// Test 41 (property) — January interval: 2026-01-01 ≡ 2026-06-30.
    #[test]
    fn january_as_of_pair_is_byte_identical(
        province in prop::sample::select(EMPLOYMENT.to_vec()),
        gross_cents in 1u64..=5_000_000u64,
        periods in prop::sample::select(PayPeriod::LEGAL.to_vec()),
        claim in 0u8..=1u8,
    ) {
        let jan = blank_request(province, "2026-01-01", periods, cents_money(gross_cents), claim);
        let mut june = jan.clone();
        june.as_of = date("2026-06-30");
        let a = serde_json::to_vec(&calculate(&jan, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        let b = serde_json::to_vec(&calculate(&june, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        prop_assert_eq!(a, b);
    }
}

#[test]
fn quebec_is_not_in_the_employment_set() {
    assert!(!EMPLOYMENT.contains(&Province::Qc));
    let err = calculate(
        &blank_request(Province::Qc, "2026-01-01", 52, money("1000.00"), 1),
        &EMBEDDED_REGISTRY,
    )
    .unwrap_err();
    assert!(matches!(err, EngineError::JurisdictionNotSupported { .. }));
}

#[test]
fn both_rule_sets_expose_thirteen_bracket_tables() {
    for set in [
        load_ruleset_2026_01_01().unwrap(),
        load_ruleset_2026_07_01().unwrap(),
    ] {
        assert_eq!(set.jurisdictions.len(), 13, "{}", set.rule_set_version);
        assert!(set
            .jurisdictions
            .contains_key(&JurisdictionCode("FED".to_string())));
        for province in EMPLOYMENT {
            if *province == Province::OutsideCanada {
                continue;
            }
            let j = set
                .jurisdictions
                .get(&JurisdictionCode(province.as_str().to_string()))
                .unwrap_or_else(|| panic!("{} missing {}", set.rule_set_version, province));
            match &j.brackets {
                OptionScoped::Both(brackets)
                | OptionScoped::PerOption {
                    option1: brackets, ..
                } => {
                    assert!(
                        brackets.len() > 1,
                        "{} {} needs more than the zero threshold",
                        set.rule_set_version,
                        province
                    );
                }
            }
        }
    }
}
