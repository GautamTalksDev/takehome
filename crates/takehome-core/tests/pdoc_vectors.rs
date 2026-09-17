//! Integration tests: CRA PDOC oracle vectors (M1 gate).
//!
//! Expected amounts are what PDOC returned — never engine output.
//! Map PDOC result lines onto engine fields before writing `expected`
//! (`Federal tax` + `Additional tax` → `federal_tax`; PDOC total includes U1).
//! On mismatch of identically specified inputs: leave expectations alone and
//! document in CONFORMANCE.md §Disagreements. A mis-mapped field is a defect,
//! not a disagreement — fix the mapping and re-run.

use serde::Deserialize;
use std::path::PathBuf;
use takehome_core::request::Request;
use takehome_core::response::Response;
use takehome_core::rules::loader::EMBEDDED_REGISTRY;
use takehome_core::{calculate, Money};

#[derive(Debug, Deserialize)]
struct VectorFile {
    vectors: Vec<PdocVector>,
}

#[derive(Debug, Deserialize)]
struct PdocVector {
    id: String,
    #[allow(dead_code)]
    description: String,
    request: serde_json::Value,
    expected: ExpectedAmounts,
    oracle: OracleMeta,
    #[serde(default)]
    #[allow(dead_code)]
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedAmounts {
    federal_tax: String,
    provincial_tax: String,
    cpp: String,
    cpp2: String,
    ei: String,
    total_deductions: String,
    net_pay: String,
}

impl ExpectedAmounts {
    fn is_pending(&self) -> bool {
        [
            self.federal_tax.as_str(),
            self.provincial_tax.as_str(),
            self.cpp.as_str(),
            self.cpp2.as_str(),
            self.ei.as_str(),
            self.total_deductions.as_str(),
            self.net_pay.as_str(),
        ]
        .contains(&"PENDING_PDOC")
    }
}

#[derive(Debug, Deserialize)]
struct OracleMeta {
    source: String,
    #[allow(dead_code)]
    url: String,
    retrieved_at: String,
    #[serde(default)]
    #[allow(dead_code)]
    pdoc_version_string: Option<String>,
    /// Calendar edition live PDOC was serving at capture (may differ from case as_of).
    observed_edition: String,
    #[serde(default)]
    #[allow(dead_code)]
    rule_set_version: Option<String>,
    #[allow(dead_code)]
    browser: String,
    operator: String,
    #[serde(default)]
    #[allow(dead_code)]
    screenshot_sha256: Option<String>,
}

fn vector_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/pdoc_ontario_2026_01.json")
}

fn money(s: &str) -> Money {
    Money::parse(s).unwrap_or_else(|e| panic!("bad money {s:?}: {e}"))
}

fn field_mismatch(id: &str, field: &str, got: &Money, expected: &str) -> Option<String> {
    let exp = money(expected);
    if got == &exp {
        None
    } else {
        Some(format!("{id}.{field}: engine={got} pdoc={exp}"))
    }
}

#[test]
fn twenty_pdoc_ontario_vectors_match_to_the_cent() {
    let raw = std::fs::read_to_string(vector_path()).expect("read vector file");
    let file: VectorFile = serde_json::from_str(&raw).expect("parse vector file");
    assert_eq!(
        file.vectors.len(),
        20,
        "M1 exit requires exactly twenty Ontario PDOC vectors"
    );

    let mut pending = Vec::new();
    let mut compared = 0usize;
    let mut mismatches = Vec::new();

    for v in &file.vectors {
        assert_eq!(v.oracle.source, "CRA PDOC");
        assert!(
            !v.oracle.observed_edition.is_empty(),
            "vector {}: oracle.observed_edition required (provenance)",
            v.id
        );
        assert!(
            matches!(v.oracle.operator.as_str(), "manual" | "pdoc-oracle-harness"),
            "vector {}: bad operator {}",
            v.id,
            v.oracle.operator
        );
        if v.expected.is_pending() {
            pending.push(v.id.clone());
            continue;
        }
        assert!(
            !v.oracle.retrieved_at.is_empty() && v.oracle.retrieved_at != "PENDING",
            "vector {}: filled expected requires oracle.retrieved_at",
            v.id
        );

        let req_json = serde_json::to_string(&v.request).unwrap();
        let req = Request::from_json(&req_json).unwrap_or_else(|e| {
            panic!(
                "vector {}: request deserialise failed: {e}\n{}",
                v.id, req_json
            )
        });
        let resp: Response = calculate(&req, &EMBEDDED_REGISTRY)
            .unwrap_or_else(|e| panic!("vector {}: calculate failed: {e}", v.id));

        for m in [
            field_mismatch(
                &v.id,
                "federal_tax",
                &resp.employee.federal_tax,
                &v.expected.federal_tax,
            ),
            field_mismatch(
                &v.id,
                "provincial_tax",
                &resp.employee.provincial_tax,
                &v.expected.provincial_tax,
            ),
            field_mismatch(&v.id, "cpp", &resp.employee.cpp, &v.expected.cpp),
            field_mismatch(&v.id, "cpp2", &resp.employee.cpp2, &v.expected.cpp2),
            field_mismatch(&v.id, "ei", &resp.employee.ei, &v.expected.ei),
            field_mismatch(
                &v.id,
                "total_deductions",
                &resp.employee.total_deductions,
                &v.expected.total_deductions,
            ),
            field_mismatch(
                &v.id,
                "net_pay",
                &resp.employee.net_pay,
                &v.expected.net_pay,
            ),
        ]
        .into_iter()
        .flatten()
        {
            mismatches.push(m);
        }
        compared += 1;
    }

    assert!(
        compared >= 1,
        "need at least one PDOC-filled vector; all still PENDING_PDOC"
    );
    eprintln!("pdoc_vectors: {compared} / 20 compared; pending={pending:?}");
    if !mismatches.is_empty() {
        panic!(
            "{} field mismatch(es) vs PDOC — do NOT edit expected to match the engine. \
             If inputs were identical, document in CONFORMANCE.md §Disagreements. \
             If a field was mis-mapped, fix the mapping and re-run:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }
    assert!(
        pending.is_empty(),
        "M1 gate incomplete: {} vectors still PENDING_PDOC (fill from PDOC only): {pending:?}",
        pending.len()
    );
}
