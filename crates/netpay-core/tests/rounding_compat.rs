//! `rounding_compat: pdoc` must not silently alias `t4127` (ADR-003).
//!
//! Until finding 002 names PDOC’s midpoint condition, the `pdoc` arm is a typed
//! not-implemented error. The field stays on the wire; the promise does not.

use netpay_core::request::{Request, RequestError, RoundingCompat};
use netpay_core::rules::loader::load_embedded_registry;
use netpay_core::{calculate, EngineError};

#[test]
fn rounding_compat_pdoc_deserializes_but_calculate_is_not_implemented() {
    let req = Request::from_json(
        r#"{
            "as_of": "2026-07-01",
            "province": "AB",
            "pay_period": 10,
            "gross_pay": "11704.49",
            "federal_claim_code": 0,
            "provincial_claim_code": 0,
            "rounding_compat": "pdoc"
        }"#,
    )
    .expect("pdoc is a legal wire value");
    assert_eq!(req.rounding_compat, RoundingCompat::Pdoc);

    let reg = load_embedded_registry().unwrap();
    let err = calculate(&req, &reg).expect_err("pdoc arm must not succeed as t4127");
    assert!(
        matches!(
            err,
            EngineError::Request(RequestError::RoundingCompatPdocNotImplemented)
        ),
        "got {err:?}"
    );
}

#[test]
fn rounding_compat_t4127_default_still_calculates() {
    let req = Request::from_json(
        r#"{
            "as_of": "2026-07-01",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "federal_claim_code": 1,
            "provincial_claim_code": 1
        }"#,
    )
    .unwrap();
    assert_eq!(req.rounding_compat, RoundingCompat::T4127);
    let reg = load_embedded_registry().unwrap();
    calculate(&req, &reg).expect("t4127 default remains implemented");
}
