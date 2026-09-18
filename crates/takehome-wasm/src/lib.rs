//! WASM JSON surface over [`takehome_core`] (spec §5.4 / §5.5).
//!
//! Rule data is already `include_str!` inside core, so the module calculates
//! with no network after the wasm bytes themselves have loaded.

use takehome_core::{
    calculate_wire, diff_rule_sets_json, jurisdictions_json, rule_set_versions_json,
    ENGINE_BUILD_SHA256,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// `calculate(requestJson) -> string`: a [`takehome_core::Response`] or an error object (spec §9.3).
///
/// Never panics and never throws into the host. Malformed JSON is
/// `{"error":{"code":"malformed_json","message":...}}`.
#[must_use]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn calculate(request_json: &str) -> String {
    calculate_wire(request_json)
}

/// `GET /v1/jurisdictions` body, including Quebec as unsupported (spec §9.2).
#[must_use]
#[cfg_attr(
    target_arch = "wasm32",
    wasm_bindgen(js_name = listJurisdictions)
)]
pub fn list_jurisdictions() -> String {
    jurisdictions_json()
}

/// Embedded T4127 rule-set editions in coverage order (spec §9.1).
#[must_use]
#[cfg_attr(
    target_arch = "wasm32",
    wasm_bindgen(js_name = listRuleSetVersions)
)]
pub fn list_rule_set_versions() -> String {
    rule_set_versions_json()
}

/// Field-by-field comparison of two embedded T4127 editions (spec §12.3).
#[must_use]
#[cfg_attr(
    target_arch = "wasm32",
    wasm_bindgen(js_name = diffRuleSets)
)]
pub fn diff_rule_sets(from: &str, to: &str) -> String {
    diff_rule_sets_json(from, to)
}

/// Source digest of `takehome-core` (spec §5.4). Same value as native [`ENGINE_BUILD_SHA256`].
#[must_use]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(js_name = engineBuildSha))]
pub fn engine_build_sha() -> String {
    ENGINE_BUILD_SHA256.to_string()
}
