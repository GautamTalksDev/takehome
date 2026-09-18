//! Native tests of the WASM crate's JSON surface (spec §5.4 / §5.5).
//!
//! The Node suite in `packages/takehome-js` is the one that instantiates the
//! wasm32 module. These tests lock the same functions the bindgen exports wrap.

use serde_json::Value;
use takehome_wasm::{
    calculate, diff_rule_sets, engine_build_sha, list_jurisdictions, list_rule_set_versions,
};

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("not JSON ({e}): {body}"))
}

#[test]
fn calculate_never_panics_on_malformed_json() {
    let body = calculate("{{{{");
    let v = parse(&body);
    assert_eq!(v["error"]["code"], "malformed_json");
    assert!(v.get("employee").is_none());
}

#[test]
fn calculate_matches_core_wire_bytes() {
    let request = r#"{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}"#;
    assert_eq!(calculate(request), takehome_core::calculate_wire(request));
}

#[test]
fn engine_build_sha_matches_core() {
    assert_eq!(engine_build_sha(), takehome_core::ENGINE_BUILD_SHA256);
    assert_eq!(engine_build_sha().len(), 64);
    assert!(engine_build_sha().chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn jurisdictions_listing_names_quebec_unsupported() {
    let v = parse(&list_jurisdictions());
    let rows = v["jurisdictions"].as_array().expect("jurisdictions array");
    let qc = rows
        .iter()
        .find(|row| row["code"] == "QC")
        .expect("QC listed");
    assert_eq!(qc["supported"], false);
}

#[test]
fn rule_set_versions_are_the_embedded_editions() {
    let v = parse(&list_rule_set_versions());
    let versions: Vec<&str> = v["rule_set_versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, ["2026-01-01", "2026-07-01", "2027-01-01"]);
}

#[test]
fn diff_rule_sets_jan_to_jul_names_bc_nl_pe() {
    let v = parse(&diff_rule_sets("2026-01-01", "2026-07-01"));
    let changed: Vec<&str> = v["changed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row.as_str().unwrap())
        .collect();
    assert_eq!(changed, ["BC", "NL", "PE"]);
}
