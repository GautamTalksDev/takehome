//! Invert M-003 midpoint-down provincial samples (same method as D-007).
//!
//! Reconstruct the T4/V1/S/K5P combination that yields PDOC, then name which
//! engine intermediate sits outside that window. No IEEE floats.

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use std::str::FromStr;
use takehome_core::decimal::{Money, Ratio};
use takehome_core::rules::loader::load_embedded_registry;
use takehome_core::{calculate, Request};

/// Provincial +1¢ vectors from the live AB queue (claim 0 and claim 1; P=10,12).
const CASES: &[(&str, u16, u8, u8, &str)] = &[
    ("11704.49", 10, 0, 0, "999.55"),
    ("11704.51", 10, 1, 1, "817.40"),
    ("15425.91", 10, 0, 0, "1367.97"),
    ("18144.00", 10, 1, 1, "1505.73"),
    ("24681.29", 10, 1, 1, "2341.64"),
    ("6120.01", 10, 0, 0, "454.15"),
    ("12854.91", 12, 1, 1, "988.18"),
    ("15425.91", 12, 0, 0, "1442.91"),
    ("20567.75", 12, 0, 0, "2103.16"),
    ("21540.17", 12, 0, 0, "2235.92"),
    ("30851.66", 12, 1, 1, "3374.70"),
];

fn money(s: &str) -> Money {
    Money::parse(s).unwrap()
}

fn ratio_of(m: Money) -> Ratio {
    m.checked_div(money("1")).unwrap()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn ratio_dec(r: Ratio) -> Decimal {
    dec(&r.to_string())
}

fn half_up(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

fn bankers(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointNearestEven)
}

fn trunc_cent(d: Decimal) -> Decimal {
    d.trunc_with_scale(2)
}

/// Exact half-cent at tax scale: fractional part of (value × 100) is 1/2.
fn is_exact_half_cent(d: Decimal) -> bool {
    let scaled = d * dec("100");
    (scaled - scaled.trunc()).abs() == dec("0.5")
}

/// T2 values whose decimal half-up period line equals `pdoc_period`.
/// Half-up: round(x) = p  iff  x ∈ [p − 0.005, p + 0.005).
fn t2_window_for_period(pdoc_period: &str, p: u16) -> (Decimal, Decimal) {
    let period = dec(pdoc_period);
    let p_n = Decimal::from(p);
    let lo = (period - dec("0.005")) * p_n;
    let hi = (period + dec("0.005")) * p_n;
    (lo, hi)
}

#[allow(dead_code)]
struct Row {
    gross: String,
    p: u16,
    claim: u8,
    pdoc: String,
    engine_period: String,
    t4: String,
    t2: String,
    v1: String,
    s: String,
    k5p: String,
    k1p: String,
    k2p: String,
    unrounded_t4: String,
    t2_over_p: String,
    t2_half_cent: bool,
    t2_in_pdoc_window: bool,
    unrounded_period_is_pdoc: bool,
    t4_bankers_period_is_pdoc: bool,
    t4_trunc_period_is_pdoc: bool,
    t4_minus_cent_period_is_pdoc: bool,
    k5p_plus_cent_period_is_pdoc: bool,
}

fn invert_one(gross: &str, p: u16, fc: u8, pc: u8, pdoc: &str) -> Row {
    let reg = load_embedded_registry().unwrap();
    let req: Request = serde_json::from_value(serde_json::json!({
        "as_of": "2026-07-01",
        "province": "AB",
        "pay_period": p,
        "gross_pay": gross,
        "federal_claim_code": fc,
        "provincial_claim_code": pc,
        "calculation_option": "option1"
    }))
    .unwrap();
    let resp = calculate(&req, &reg).unwrap();
    let b = &resp.breakdown;
    let p_money = money(&p.to_string());
    let one = money("1");

    let unrounded =
        b.a.checked_mul_rate(b.v)
            .unwrap()
            .checked_sub(b.kp)
            .unwrap()
            .checked_sub(b.k1p)
            .unwrap()
            .checked_sub(b.k2p)
            .unwrap()
            .checked_sub(b.k3p)
            .unwrap()
            .checked_sub(b.k4p)
            .unwrap()
            .checked_sub(b.k5p)
            .unwrap()
            .floor_at_zero();
    let unrounded_d = ratio_dec(unrounded.checked_div(one).unwrap());
    let t2_over_p = ratio_dec(b.t2.checked_div(p_money).unwrap());
    let (lo, hi) = t2_window_for_period(pdoc, p);
    let t2_d = ratio_dec(ratio_of(b.t2));

    let unrounded_period = half_up(unrounded_d / Decimal::from(p));
    let bankers_t4 = bankers(unrounded_d);
    let bankers_period = half_up(bankers_t4 / Decimal::from(p));
    let trunc_t4 = trunc_cent(unrounded_d);
    let trunc_period = half_up(trunc_t4 / Decimal::from(p));
    let t4_minus = ratio_dec(ratio_of(b.t4.checked_sub(money("0.01")).unwrap()));
    let t4_minus_period = half_up(t4_minus / Decimal::from(p));
    let t2_with_k5p = ratio_dec(ratio_of(
        b.t2.checked_sub(money("0.01")).unwrap(), // K5P +1¢ lowers T4/T2
    ));
    let k5p_plus_period = half_up(t2_with_k5p / Decimal::from(p));

    Row {
        gross: gross.to_string(),
        p,
        claim: pc,
        pdoc: pdoc.to_string(),
        engine_period: resp.employee.provincial_tax.to_string(),
        t4: b.t4.to_string(),
        t2: b.t2.to_string(),
        v1: b.v1.to_string(),
        s: b.s.to_string(),
        k5p: b.k5p.to_string(),
        k1p: b.k1p.to_string(),
        k2p: b.k2p.to_string(),
        unrounded_t4: unrounded_d.to_string(),
        t2_over_p: t2_over_p.to_string(),
        t2_half_cent: is_exact_half_cent(t2_over_p),
        t2_in_pdoc_window: t2_d >= lo && t2_d < hi,
        unrounded_period_is_pdoc: unrounded_period == dec(pdoc),
        t4_bankers_period_is_pdoc: bankers_period == dec(pdoc),
        t4_trunc_period_is_pdoc: trunc_period == dec(pdoc),
        t4_minus_cent_period_is_pdoc: t4_minus_period == dec(pdoc),
        k5p_plus_cent_period_is_pdoc: k5p_plus_period == dec(pdoc),
    }
}

#[test]
fn invert_alberta_provincial_from_pdoc_period() {
    let mut rows = Vec::new();
    for &(gross, p, fc, pc, pdoc) in CASES {
        let row = invert_one(gross, p, fc, pc, pdoc);
        eprintln!(
            "gross={gross} P={p} claim={pc} pdoc={pdoc} eng={} T4={} T2={} unrounded_T4={} T2/P={} half={} window={} unrounded→pdoc={} bankers→pdoc={} trunc→pdoc={} T4-1¢→pdoc={} K5P+1¢→pdoc={} V1={} S={} K5P={} K1P={} K2P={}",
            row.engine_period,
            row.t4,
            row.t2,
            row.unrounded_t4,
            row.t2_over_p,
            row.t2_half_cent,
            row.t2_in_pdoc_window,
            row.unrounded_period_is_pdoc,
            row.t4_bankers_period_is_pdoc,
            row.t4_trunc_period_is_pdoc,
            row.t4_minus_cent_period_is_pdoc,
            row.k5p_plus_cent_period_is_pdoc,
            row.v1,
            row.s,
            row.k5p,
            row.k1p,
            row.k2p,
        );
        assert_eq!(row.t4, row.t2, "AB has no V1/S/LCP: T2 must equal T4");
        assert_eq!(row.v1, "0.00");
        assert_eq!(row.s, "0.00");
        assert_ne!(row.engine_period, row.pdoc);
        rows.push(row);
    }

    let n = rows.len();
    let count = |f: fn(&Row) -> bool| rows.iter().filter(|r| f(r)).count();
    eprintln!(
        "SUMMARY n={n} half_cent={} in_window={} unrounded={} bankers={} trunc={} T4-1¢={} K5P+1¢={} k5p_always_zero={}",
        count(|r| r.t2_half_cent),
        count(|r| r.t2_in_pdoc_window),
        count(|r| r.unrounded_period_is_pdoc),
        count(|r| r.t4_bankers_period_is_pdoc),
        count(|r| r.t4_trunc_period_is_pdoc),
        count(|r| r.t4_minus_cent_period_is_pdoc),
        count(|r| r.k5p_plus_cent_period_is_pdoc),
        count(|r| r.k5p == "0.00"),
    );

    // The residual must be the same shape on every provincial mismatch.
    assert_eq!(
        count(|r| r.k5p == "0.00"),
        n,
        "K5P is dead on claim-code forms; cannot be the residual"
    );
    assert_eq!(
        count(|r| r.t2 == r.t4 && r.v1 == "0.00" && r.s == "0.00"),
        n,
        "T4/V1/S assembly is T2=T4 with V1=S=0 — not a 1¢ credit/surtax miss"
    );
    assert_eq!(
        count(|r| r.t2_half_cent),
        n,
        "residual is Step 6 T2/P exact half-cent on every provincial miss"
    );
    assert_eq!(
        count(|r| r.t2_in_pdoc_window),
        0,
        "engine T2 sits on the exclusive upper bound of the PDOC window (*.x5 / P)"
    );
    assert!(
        count(|r| r.unrounded_period_is_pdoc) < n,
        "skipping T4 rounding does not uniquely produce PDOC"
    );
    assert_eq!(
        count(|r| r.t4_minus_cent_period_is_pdoc),
        n,
        "T4−1¢ is tautological at a half-cent T2/P; names period rounding, not a 1¢ T4 bug"
    );
}
