//! Python JSON surface over [`takehome_core`] (spec §5.4).
//!
//! `calculate` never raises into Python on malformed JSON; it returns the same
//! error object as the WASM binding.

use pyo3::prelude::*;

/// T4127 deduction. JSON in; JSON [`takehome_core::Response`] or error object out (spec §9.3).
#[pyfunction]
fn calculate(request_json: &str) -> String {
    takehome_core::calculate_wire(request_json)
}

/// `GET /v1/jurisdictions` body, including Quebec as unsupported (spec §9.2).
#[pyfunction]
fn list_jurisdictions() -> String {
    takehome_core::jurisdictions_json()
}

/// Embedded T4127 rule-set editions in coverage order (spec §9.1).
#[pyfunction]
fn list_rule_set_versions() -> String {
    takehome_core::rule_set_versions_json()
}

/// Source digest of `takehome-core` (spec §5.4). Same value as the native engine.
#[pyfunction]
fn engine_build_sha() -> String {
    takehome_core::ENGINE_BUILD_SHA256.to_string()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(calculate, m)?)?;
    m.add_function(wrap_pyfunction!(list_jurisdictions, m)?)?;
    m.add_function(wrap_pyfunction!(list_rule_set_versions, m)?)?;
    m.add_function(wrap_pyfunction!(engine_build_sha, m)?)?;
    Ok(())
}
