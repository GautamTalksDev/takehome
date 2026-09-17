//! JSON wire contract used by every runtime (spec §5.4 / §9.3).
//!
//! Bindings must not invent a second request/response shape. WASM, Python, and
//! the CLI call [`takehome_core::calculate_wire`].

use serde_json::Value;
use takehome_core::rules::loader::EMBEDDED_REGISTRY;
use takehome_core::{calculate_wire, list_rule_set_versions, ENGINE_BUILD_SHA256};

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("wire output is not JSON ({e}): {body}"))
}

fn error_code(body: &str) -> String {
    let v = parse(body);
    v["error"]["code"]
        .as_str()
        .unwrap_or_else(|| panic!("missing error.code in {body}"))
        .to_string()
}

#[test]
fn malformed_json_returns_structured_error_not_a_success_body() {
    for input in ["", "{", "null", "[]", "not json", "\u{0000}"] {
        let body = calculate_wire(input);
        let v = parse(&body);
        assert!(
            v.get("employee").is_none(),
            "{input:?} must not look like a response"
        );
        assert_eq!(error_code(&body), "malformed_json", "{input:?}");
        assert!(
            v["error"]["message"]
                .as_str()
                .is_some_and(|m| !m.is_empty()),
            "{input:?} needs a message"
        );
    }
}

#[test]
fn unknown_field_is_invalid_request_and_names_the_field() {
    let body = calculate_wire(
        r#"{"as_of":"2026-03-15","province":"ON","pay_period":26,"gross_pay":"2500.00","typo_gross":"1.00"}"#,
    );
    assert_eq!(error_code(&body), "malformed_json");
    assert!(
        parse(&body)["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("typo_gross")),
        "{}",
        body
    );
}

#[test]
fn quebec_is_typed_refusal_on_the_wire() {
    let body = calculate_wire(
        r#"{"as_of":"2026-01-15","province":"QC","pay_period":52,"gross_pay":"1000.00"}"#,
    );
    assert_eq!(error_code(&body), "jurisdiction_not_supported");
    let parsed = parse(&body);
    let msg = parsed["error"]["message"].as_str().unwrap();
    assert!(msg.contains("QC"), "{msg}");
    assert!(msg.contains("Quebec"), "{msg}");
}

#[test]
fn ontario_weekly_1000_wire_matches_native_calculate_bytes() {
    let request = r#"{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1,"cpp_months":12}"#;
    let wire = calculate_wire(request);
    let req = takehome_core::Request::from_json(request).unwrap();
    let resp = takehome_core::calculate(&req, &EMBEDDED_REGISTRY).unwrap();
    let native = serde_json::to_string(&resp).unwrap();
    assert_eq!(wire, native);
    let v = parse(&wire);
    assert_eq!(v["employee"]["net_pay"], "800.79");
    assert_eq!(v["engine_build_sha256"], ENGINE_BUILD_SHA256);
}

#[test]
fn calculate_wire_is_byte_identical_across_calls() {
    let request = r#"{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00"}"#;
    let a = calculate_wire(request);
    let b = calculate_wire(request);
    assert_eq!(a, b);
}

#[test]
fn list_rule_set_versions_covers_both_2026_editions() {
    let listing = list_rule_set_versions();
    let versions: Vec<&str> = listing
        .rule_set_versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, ["2026-01-01", "2026-07-01"]);
    assert_eq!(
        listing.rule_set_versions[0].effective_from.to_string(),
        "2026-01-01"
    );
    assert_eq!(
        listing.rule_set_versions[0]
            .effective_to
            .expect("january is closed")
            .to_string(),
        "2026-07-01"
    );
    assert!(listing.rule_set_versions[1].effective_to.is_none());
}
