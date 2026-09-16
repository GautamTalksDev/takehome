//! July 2026 delta ingest (spec §21.9). Tests 14–19.

use netpay_core::rules::schema::{
    BasicPersonalAmount, CalculationOption, Jurisdiction, OptionScoped,
};
use netpay_core::{Money, Rate};
use netpay_ingest::{ingest_archive_with, parse_bracket_table, IngestOptions, TABLE_8_1};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(rel)
}

fn unique_temp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("netpay-ingest-july-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("{}: {e}", path.display());
    }))
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn load_jurisdiction(path: &Path) -> Jurisdiction {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("{}: {e}", path.display());
    }))
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Test 14 — July is a delta: explicit marker, overlay produces 13 jurisdictions,
/// overlay-disabled lists the ten missing. Detection is not a row-count check.
#[test]
fn july_delta_overlay_and_missing_jurisdictions_without_overlay() {
    let archive = workspace_data("sources/t4127-jul-2026");
    let meta_raw = fs::read_to_string(archive.join("archive.json")).unwrap();
    let meta: Value = serde_json::from_str(&meta_raw).unwrap();
    assert_eq!(meta["edition_kind"], "delta");
    let marker = meta["delta_marker"].as_str().expect("delta_marker");
    assert!(
        marker.contains("122nd edition") && marker.contains("have not been reproduced"),
        "explicit CRA delta marker, not a row count: {marker}"
    );
    assert_eq!(
        meta["delta_jurisdictions"],
        serde_json::json!(["BC", "NL", "PE"])
    );

    let csv = fs::read(archive.join("rates-income-thresholds-constants-26e.csv")).unwrap();
    let text = netpay_ingest::decode_bytes(&csv).text;
    let parsed = parse_bracket_table("rates-income-thresholds-constants-26e.csv", &text).unwrap();
    assert_eq!(
        parsed.len(),
        TABLE_8_1.len(),
        "July Table 8.1 reprints all {} jurisdictions; row count would lie",
        TABLE_8_1.len()
    );

    let out = unique_temp("overlay");
    let report = ingest_archive_with(
        &archive,
        &out,
        IngestOptions {
            base_rules_dir: Some(&workspace_data("rules/2026-01-01")),
            overlay: true,
        },
    )
    .unwrap_or_else(|e| panic!("overlay ingest: {e}"));
    assert_eq!(report.effective_from, "2026-07-01");

    let manifest = read_json(&out.join("manifest.json"));
    assert_eq!(manifest["edition_kind"], "delta");
    assert_eq!(manifest["delta_marker"], marker);
    let origin = manifest["jurisdiction_origin"].as_object().unwrap();
    for code in ["BC", "NL", "PE"] {
        assert_eq!(origin[code], "delta", "{code}");
    }
    for code in ["FED", "AB", "MB", "NB", "NS", "NT", "NU", "ON", "SK", "YT"] {
        assert_eq!(origin[code], "inherited", "{code}");
    }
    assert_eq!(origin.len(), 13);

    let disabled = unique_temp("no-overlay");
    let err = ingest_archive_with(
        &archive,
        &disabled,
        IngestOptions {
            base_rules_dir: None,
            overlay: false,
        },
    )
    .expect_err("delta without overlay must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("missing jurisdictions"),
        "must list missing jurisdictions, got {msg}"
    );
    for code in ["FED", "AB", "MB", "NB", "NS", "NT", "NU", "ON", "SK", "YT"] {
        assert!(
            msg.contains(code),
            "missing list must include {code}: {msg}"
        );
    }
    for code in ["BC", "NL", "PE"] {
        let after = msg.split("missing jurisdictions:").nth(1).unwrap_or("");
        assert!(
            !after.split(',').any(|s| s.trim() == code),
            "{code} is in the delta, should not be listed missing: {msg}"
        );
    }

    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_dir_all(&disabled);
}

/// Test 15 — BC OptionScoped shapes on ingest output.
#[test]
fn july_bc_option_scoped_from_ingest() {
    let bc = load_jurisdiction(&workspace_data("rules/2026-07-01/bc.json"));
    let OptionScoped::PerOption { option1, option2 } = &bc.brackets else {
        panic!("BC brackets must be PerOption, not Both");
    };
    assert_eq!(option1[0].rate, Rate::parse("0.0614").unwrap());
    assert!(option1[0].prorated);
    assert_eq!(option2[0].rate, Rate::parse("0.0560").unwrap());
    assert!(!option2[0].prorated);
    let OptionScoped::PerOption {
        option1: r1,
        option2: r2,
    } = &bc.lowest_rate
    else {
        panic!("lowest_rate PerOption");
    };
    assert_eq!(*r1, Rate::parse("0.0614").unwrap());
    assert_eq!(*r2, Rate::parse("0.0560").unwrap());
    let OptionScoped::PerOption {
        option1: s1,
        option2: s2,
    } = bc.tax_reduction.as_ref().unwrap()
    else {
        panic!("tax_reduction PerOption");
    };
    assert_eq!(s1.basic, Money::parse("805.00").unwrap());
    assert_eq!(s2.basic, Money::parse("690.00").unwrap());
    assert_eq!(s1.dependant, Money::parse("44952.00").unwrap());
    assert_eq!(s2.dependant, Money::parse("44952.00").unwrap());
}

/// Test 16 — NL brackets unchanged; BPA 15000 vs 13094.
#[test]
fn july_nl_bpa_per_option_brackets_both() {
    let jan = load_jurisdiction(&workspace_data("rules/2026-01-01/nl.json"));
    let july = load_jurisdiction(&workspace_data("rules/2026-07-01/nl.json"));
    assert_eq!(
        jan.brackets.get(CalculationOption::Option1),
        july.brackets.get(CalculationOption::Option1)
    );
    assert!(matches!(july.brackets, OptionScoped::Both(_)));
    let OptionScoped::PerOption { option1, option2 } = &july.basic_personal_amount else {
        panic!("NL BPA PerOption");
    };
    match option1 {
        BasicPersonalAmount::Fixed { amount } => {
            assert_eq!(*amount, Money::parse("15000.00").unwrap())
        }
        other => panic!("{other:?}"),
    }
    match option2 {
        BasicPersonalAmount::Fixed { amount } => {
            assert_eq!(*amount, Money::parse("13094.00").unwrap())
        }
        other => panic!("{other:?}"),
    }
}

/// Test 17 — PE January 5 brackets, July 6.
#[test]
fn july_pe_gains_sixth_bracket_on_ingest() {
    let jan = load_jurisdiction(&workspace_data("rules/2026-01-01/pe.json"));
    let july = load_jurisdiction(&workspace_data("rules/2026-07-01/pe.json"));
    assert_eq!(jan.brackets.get(CalculationOption::Option1).len(), 5);
    let OptionScoped::PerOption { option1, option2 } = &july.brackets else {
        panic!("PE July brackets PerOption");
    };
    assert_eq!(option1.len(), 6);
    assert_eq!(option2.len(), 6);
    assert_eq!(option1[5].threshold, Money::parse("200000.00").unwrap());
    assert_eq!(option1[5].rate, Rate::parse("0.2100").unwrap());
    assert_eq!(option1[5].constant, Money::parse("10464.00").unwrap());
    assert!(option1[5].prorated);
    assert_eq!(option2[5].rate, Rate::parse("0.2000").unwrap());
    assert_eq!(option2[5].constant, Money::parse("8464.00").unwrap());
    assert!(!option2[5].prorated);
}

/// Test 18 — ten inherited jurisdiction JSON files are byte-identical.
#[test]
fn inherited_jurisdiction_files_are_byte_identical() {
    let jan = workspace_data("rules/2026-01-01");
    let july = workspace_data("rules/2026-07-01");
    for name in [
        "federal.json",
        "ab.json",
        "mb.json",
        "nb.json",
        "ns.json",
        "nt.json",
        "nu.json",
        "on.json",
        "sk.json",
        "yt.json",
    ] {
        let a = fs::read(jan.join(name)).unwrap();
        let b = fs::read(july.join(name)).unwrap();
        assert_eq!(
            a, b,
            "{name} must be copied byte-for-byte; overlay is wrong"
        );
    }
}

/// Test 19 — BC and NL July claim-code tables differ from January; the other
/// eleven are identical.
#[test]
fn claim_codes_effective_dated_for_bc_and_nl() {
    let jan = workspace_data("rules/2026-01-01/claim-codes");
    let july = workspace_data("rules/2026-07-01/claim-codes");
    let jan_names: BTreeSet<_> = fs::read_dir(&jan)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let july_names: BTreeSet<_> = fs::read_dir(&july)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(jan_names, july_names);
    assert_eq!(jan_names.len(), 13);
    for name in &jan_names {
        let a = fs::read(jan.join(name)).unwrap();
        let b = fs::read(july.join(name)).unwrap();
        let stem = name.to_string_lossy();
        if stem == "bc.json" || stem == "nl.json" {
            assert_ne!(a, b, "{stem} July claim codes must differ from January");
        } else {
            assert_eq!(a, b, "{stem} claim codes must be identical");
        }
    }
}
