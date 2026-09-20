//! Tests 48–50: 2027-01-01 PREVIEW rule set (spec open question 5).
//!
//! The Spring Economic Update 2026 announced a CPP base-rate cut effective
//! 1 January 2027. There is no 2027 T4127 edition and no published 2027 YMPE,
//! so the set is labelled proposed — not enacted.

use takehome_core::rules::schema::RuleSetStatus;
use takehome_core::{calculate_wire, list_rule_set_versions};

fn body(as_of: &str) -> serde_json::Value {
    let request = format!(
        r#"{{"as_of":"{as_of}","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}}"#
    );
    serde_json::from_str(&calculate_wire(&request)).unwrap_or_else(|err| {
        panic!(
            "calculate_wire({as_of}) was not JSON ({err}): {}",
            calculate_wire(&request)
        );
    })
}

fn proposed_warning(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|warning| warning["code"] == "RULE_SET_PROPOSED")
}

/// 48. The 2027-01-01 directory is the newest embedded set and is proposed.
#[test]
fn listing_ships_2027_preview_as_proposed() {
    let listing = list_rule_set_versions();
    let versions: Vec<&str> = listing
        .rule_set_versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, ["2026-01-01", "2026-07-01", "2027-01-01"]);
    assert_eq!(listing.rule_set_versions[0].status, RuleSetStatus::Enacted);
    assert_eq!(listing.rule_set_versions[1].status, RuleSetStatus::Enacted);
    assert_eq!(listing.rule_set_versions[2].status, RuleSetStatus::Proposed);
    assert_eq!(
        listing.rule_set_versions[1]
            .effective_to
            .expect("july closes when the 2027 preview ships")
            .to_string(),
        "2027-01-01"
    );
    assert!(listing.rule_set_versions[2].effective_to.is_none());
}

/// 49. A 2027 as_of returns the preview and a proposed warning that names
///     2026-04-28. A 2026 as_of does not carry that warning.
#[test]
fn as_of_2027_carries_proposed_warning_naming_announcement_date() {
    let y2027 = body("2027-01-15");
    assert_eq!(y2027["rule_set_version"], "2027-01-01");
    let warning = proposed_warning(&y2027).expect("2027 must warn that the set is proposed");
    let message = warning["message"].as_str().expect("warning message");
    assert!(
        message.to_ascii_lowercase().contains("proposed"),
        "{message}"
    );
    assert!(
        message.to_ascii_lowercase().contains("not yet enacted"),
        "{message}"
    );
    assert!(message.contains("2026-04-28"), "{message}");

    let y2026 = body("2026-01-15");
    assert_eq!(y2026["rule_set_version"], "2026-01-01");
    assert!(proposed_warning(&y2026).is_none());
    let y2026_july = body("2026-07-15");
    assert_eq!(y2026_july["rule_set_version"], "2026-07-01");
    assert!(proposed_warning(&y2026_july).is_none());
}

/// 50. Ontario weekly $1000 CPP is 55.50 under 2026 rates and 53.63 under
///     2027 rates (0.0575 × 932.70, half-up). If these are equal, a 0.0495/0.0595
///     ratio is hard-coded in a formula — which is why the rule data is data.
#[test]
fn ontario_weekly_1000_cpp_follows_2027_rule_data() {
    let y2026 = body("2026-07-15");
    let y2027 = body("2027-01-15");
    assert_eq!(y2026["employee"]["cpp"], "55.50");
    assert_eq!(y2027["employee"]["cpp"], "53.63");
    assert_ne!(
        y2026["employee"]["cpp"], y2027["employee"]["cpp"],
        "2027 CPP matching 2026 means the formula is not reading the rule set"
    );
}
