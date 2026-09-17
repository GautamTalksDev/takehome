//! Documents two Alberta smoke cases where engine period provincial is +1¢ vs PDOC
//! at an exact half-cent T2/P boundary. IEEE float residue *coincidentally* matches
//! PDOC here; finding 002 is PDOC midpoint direction (not Alberta-specific, float
//! as a general rule falsified). Kept as a regression for the observed AB vectors.

use rust_decimal::Decimal;
use std::str::FromStr;
use takehome_core::decimal::Money;
use takehome_core::rounding::round_tax_to_cent;
use takehome_core::rules::loader::load_embedded_registry;
use takehome_core::{calculate, Request};

fn money(s: &str) -> Money {
    Money::parse(s).unwrap()
}

#[test]
fn alberta_half_cent_t2_over_p_matches_float_residue_not_k5p() {
    let reg = load_embedded_registry().unwrap();
    let cases = [
        ("11704.49", "9995.55", "999.56", "999.55"),
        ("15425.91", "13679.75", "1367.98", "1367.97"),
    ];
    for (gross, expect_t2, engine_period, pdoc_period) in cases {
        let req: Request = serde_json::from_value(serde_json::json!({
            "as_of": "2026-07-01",
            "province": "AB",
            "pay_period": 10,
            "gross_pay": gross,
            "federal_claim_code": 0,
            "provincial_claim_code": 0,
            "calculation_option": "option1"
        }))
        .unwrap();
        let resp = calculate(&req, &reg).unwrap();
        assert_eq!(resp.breakdown.k5p, money("0.00"), "claim 0 ⇒ K5P=0");
        assert_eq!(resp.breakdown.t2.to_string(), expect_t2);
        let p = Money::parse("10").unwrap();
        let exact = resp.breakdown.t2.checked_div(p).unwrap();
        assert_eq!(round_tax_to_cent(exact).to_string(), engine_period);
        assert_eq!(resp.employee.provincial_tax.to_string(), engine_period);

        // IEEE float path (PDOC-shaped): divide in f64, then half-up.
        let t2_f = f64::from_str(expect_t2).unwrap();
        let q = t2_f / 10.0;
        let as_dec = Decimal::from_f64_retain(q).unwrap();
        let float_rounded =
            as_dec.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
        assert_eq!(
            float_rounded.to_string(),
            pdoc_period,
            "float residue must land on PDOC period amount"
        );
        assert_ne!(engine_period, pdoc_period);
    }
}
