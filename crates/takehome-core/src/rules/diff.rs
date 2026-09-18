//! Field-by-field comparison of two embedded T4127 rule-set editions (spec §12.3).
//!
//! Public surface is [`diff_rule_sets`]. Every numeric still travels as a
//! lexical string: this module serializes already-loaded [`RuleSet`] values
//! and walks the JSON, it never parses a float.

use crate::rules::loader::EMBEDDED_REGISTRY;
use crate::rules::registry::RuleError;
use crate::rules::schema::RuleSet;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::{diff_rule_sets, RuleSetDiff};

    /// Test 47 — January 2026 → July 2026 names exactly BC, NL, and PE.
    /// July is a CRA delta; federal / CPP / EI / QPIP and the other ten
    /// jurisdictions must not appear as changed.
    #[test]
    fn jan_to_jul_2026_names_exactly_bc_nl_and_pe() {
        let diff = diff_rule_sets("2026-01-01", "2026-07-01").expect("both editions are embedded");
        assert_eq!(
            diff.changed,
            vec!["BC".to_string(), "NL".to_string(), "PE".to_string()],
            "July 2026 delta jurisdictions: {diff:?}"
        );
        assert!(
            diff.fields.contains_key("BC")
                && diff.fields.contains_key("NL")
                && diff.fields.contains_key("PE"),
            "each named jurisdiction must list the fields that moved: {diff:?}"
        );
        assert_eq!(diff.fields.len(), 3);
        assert!(
            diff.cpp.is_empty(),
            "CPP must not move mid-year: {:?}",
            diff.cpp
        );
        assert!(
            diff.ei.is_empty(),
            "EI must not move mid-year: {:?}",
            diff.ei
        );
        assert!(
            diff.qpip.is_empty(),
            "QPIP must not move mid-year: {:?}",
            diff.qpip
        );
        assert_eq!(diff.from, "2026-01-01");
        assert_eq!(diff.to, "2026-07-01");
    }

    #[test]
    fn unknown_version_names_the_version() {
        let err = diff_rule_sets("2026-01-01", "2099-01-01").expect_err("2099 is not embedded");
        let msg = err.to_string();
        assert!(
            msg.contains("2099-01-01"),
            "error must name the missing version, got {msg}"
        );
    }

    #[test]
    fn identical_editions_name_nothing() {
        let diff = diff_rule_sets("2026-01-01", "2026-01-01").expect("same edition");
        assert_eq!(diff.changed, Vec::<String>::new());
        assert!(diff.fields.is_empty());
    }

    #[test]
    fn wire_shape_matches_api_contract() {
        let diff = diff_rule_sets("2026-01-01", "2026-07-01").unwrap();
        let encoded = serde_json::to_value(&diff).unwrap();
        assert_eq!(encoded["changed"], serde_json::json!(["BC", "NL", "PE"]));
        let _typed: RuleSetDiff = serde_json::from_value(encoded).unwrap();
    }
}

/// Field-by-field delta between two rule-set versions (spec §12.3).
///
/// `changed` is the sorted list of jurisdiction codes whose tax fields
/// differ. CPP / EI / QPIP paths are reported separately so a mid-year
/// provincial delta cannot be confused with a federal parameter change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct RuleSetDiff {
    pub from: String,
    pub to: String,
    pub changed: Vec<String>,
    pub fields: BTreeMap<String, Vec<String>>,
    pub cpp: Vec<String>,
    pub ei: Vec<String>,
    pub qpip: Vec<String>,
}

/// Compare two embedded T4127 editions field by field (spec §12.3).
///
/// Identity metadata (`rule_set_version`, `effective_from`, source URL,
/// retrieval time, digest) is excluded: those always change between
/// editions. What remains is the tax content.
pub fn diff_rule_sets(from: &str, to: &str) -> Result<RuleSetDiff, RuleError> {
    let left = set_by_version(from)?;
    let right = set_by_version(to)?;
    Ok(diff_loaded(left, right))
}

/// JSON for `diffRuleSets` / `GET /v1/rules/diff`. Error object on failure.
pub fn diff_rule_sets_json(from: &str, to: &str) -> String {
    match diff_rule_sets(from, to) {
        Ok(diff) => match serde_json::to_string(&diff) {
            Ok(body) => body,
            Err(err) => crate::EngineError::Message(err.to_string()).to_wire_json(),
        },
        Err(err) => crate::EngineError::from(err).to_wire_json(),
    }
}

fn set_by_version(version: &str) -> Result<&RuleSet, RuleError> {
    EMBEDDED_REGISTRY
        .get(version)
        .ok_or_else(|| RuleError::UnknownVersion {
            version: version.to_string(),
        })
}

fn diff_loaded(from: &RuleSet, to: &RuleSet) -> RuleSetDiff {
    let left = serde_json::to_value(from).unwrap_or(Value::Null);
    let right = serde_json::to_value(to).unwrap_or(Value::Null);

    let mut fields = BTreeMap::new();
    let left_j = left.get("jurisdictions").cloned().unwrap_or(Value::Null);
    let right_j = right.get("jurisdictions").cloned().unwrap_or(Value::Null);
    let codes = json_keys(&left_j)
        .into_iter()
        .chain(json_keys(&right_j))
        .collect::<std::collections::BTreeSet<_>>();
    for code in codes {
        let mut paths = Vec::new();
        walk(
            left_j.get(&code).unwrap_or(&Value::Null),
            right_j.get(&code).unwrap_or(&Value::Null),
            "",
            &mut paths,
        );
        if !paths.is_empty() {
            fields.insert(code, paths);
        }
    }

    let changed: Vec<String> = fields.keys().cloned().collect();
    RuleSetDiff {
        from: from.rule_set_version.clone(),
        to: to.rule_set_version.clone(),
        changed,
        fields,
        cpp: component_paths(&left, &right, "cpp"),
        ei: component_paths(&left, &right, "ei"),
        qpip: component_paths(&left, &right, "qpip"),
    }
}

fn component_paths(left: &Value, right: &Value, key: &str) -> Vec<String> {
    let mut paths = Vec::new();
    walk(
        left.get(key).unwrap_or(&Value::Null),
        right.get(key).unwrap_or(&Value::Null),
        "",
        &mut paths,
    );
    paths
}

fn json_keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

fn walk(left: &Value, right: &Value, prefix: &str, out: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                match (a.get(key), b.get(key)) {
                    (Some(l), Some(r)) => walk(l, r, &next, out),
                    _ => out.push(next),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            let n = a.len().max(b.len());
            for i in 0..n {
                let next = format!("{prefix}[{i}]");
                match (a.get(i), b.get(i)) {
                    (Some(l), Some(r)) => walk(l, r, &next, out),
                    _ => out.push(next),
                }
            }
        }
        (l, r) if l == r => {}
        _ => out.push(prefix.to_string()),
    }
}
