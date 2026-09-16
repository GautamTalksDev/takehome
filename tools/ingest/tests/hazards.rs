//! Documented CRA CSV format hazards (spec §6.1 / §21.8). One test per hazard,
//! then the January 2026 round-trip that is the point of this crate.

use netpay_core::rounding::round_claim_to_dollar;
use netpay_core::rules::schema::{
    BasicPersonalAmount, Jurisdiction, JurisdictionCode, OptionScoped, RuleSet,
};
use netpay_core::Money;
use netpay_ingest::{
    cra_code_to_jurisdiction, csv_rows, decode_bytes, filename_to_jurisdiction, ingest_archive,
    parse_bracket_table, parse_other_amounts, special_token, strip_thousands, BasicCell,
    EncodingKind, SpecialToken, JURISDICTION_TO_CRA, THIRTEEN,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn fixture(name: &str) -> PathBuf {
    fixtures().join(name)
}

fn fixture_bytes(name: &str) -> Vec<u8> {
    fs::read(fixture(name)).unwrap_or_else(|e| panic!("read {name}: {e}"))
}

fn fixture_text(name: &str) -> String {
    decode_bytes(&fixture_bytes(name)).text
}

fn workspace_data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .join(rel)
}

/// Test 1 — encoding: UTF-8-with-BOM vs Windows-1252, detected per file.
/// Fixture contains a Windows-1252 accented character in a jurisdiction name
/// (`Québec`, byte 0xE9).
#[test]
fn encoding_detected_per_file_windows1252_accented_name() {
    let win = fixture_bytes("encoding-windows1252.csv");
    assert!(
        win.contains(&0xE9),
        "fixture must contain Windows-1252 é (0xE9), not UTF-8 C3 A9"
    );
    assert!(
        std::str::from_utf8(&win).is_err(),
        "fixture must not be valid UTF-8 — that would skip the hazard"
    );
    let decoded = decode_bytes(&win);
    assert_eq!(decoded.encoding, EncodingKind::Windows1252);
    assert!(
        decoded.text.contains("Québec"),
        "decoded jurisdiction name: {}",
        decoded.text
    );

    let bom = fixture_bytes("encoding-utf8-bom.csv");
    let decoded_bom = decode_bytes(&bom);
    assert_eq!(decoded_bom.encoding, EncodingKind::Utf8Bom);
    assert!(decoded_bom.text.starts_with("Jurisdiction"));
}

/// Test 2 — quoted thousands: `"1,123.07"` → `"1123.07"`.
/// Reject `"1,12.07"` and `"1,,123.07"` rather than repairing them.
#[test]
fn quoted_thousands_strips_and_rejects_malformed() {
    let text = fixture_text("numbers-quoted.csv");
    let rows = csv_rows("numbers-quoted.csv", &text).expect("fixture csv");
    let mut by_label = BTreeMap::new();
    for (_line, fields, _content) in rows.into_iter().skip(1) {
        by_label.insert(fields[0].clone(), fields[1].clone());
    }
    assert_eq!(
        strip_thousands(&by_label["EI_max"]).expect("quoted thousands"),
        "1123.07"
    );
    assert_eq!(strip_thousands("1,123.07").unwrap(), "1123.07");
    let short = strip_thousands(&by_label["bad_short"]).expect_err("1,12.07 must not repair");
    assert!(
        short.to_string().contains("1,12.07"),
        "error names the bad token: {short}"
    );
    let empty = strip_thousands(&by_label["bad_empty"]).expect_err("1,,123.07 must not repair");
    assert!(
        empty.to_string().contains("1,,123.07"),
        "error names the bad token: {empty}"
    );
}

/// Test 3 — `nv` is Nunavut (not `nu`), `pei` is PEI (not `pe`).
/// Explicit bidirectional map of all 13; unmapped is a hard error.
#[test]
fn province_codes_map_all_thirteen_and_reject_unmapped() {
    let text = fixture_text("provinces.csv");
    let rows = csv_rows("provinces.csv", &text).expect("fixture csv");
    let mut seen = Vec::new();
    for (_line, fields, _content) in rows.into_iter().skip(1) {
        let cra = fields[0].as_str();
        let canonical = fields[1].as_str();
        assert_eq!(
            cra_code_to_jurisdiction(cra).expect(cra),
            canonical,
            "CRA token {cra}"
        );
        seen.push(canonical.to_string());
        let reverse = JURISDICTION_TO_CRA
            .iter()
            .find(|(code, _)| *code == canonical)
            .map(|(_, token)| *token)
            .expect(canonical);
        assert_eq!(reverse, cra, "reverse map {canonical} → {cra}");
        assert_eq!(
            cra_code_to_jurisdiction(canonical).expect(canonical),
            canonical,
            "table-cell form {canonical}"
        );
        let filename = format!("cc-{cra}-01-26e.csv");
        assert_eq!(
            filename_to_jurisdiction(&filename).expect(&filename),
            canonical
        );
    }
    assert_eq!(seen.len(), 13, "fixture must list every province/territory");
    for code in THIRTEEN {
        assert!(seen.iter().any(|s| s == code), "missing {code}");
    }

    let unmapped = fixture_text("provinces-unmapped.csv");
    let rows = csv_rows("provinces-unmapped.csv", &unmapped).expect("unmapped fixture");
    for (_line, fields, _content) in rows.into_iter().skip(1) {
        let code = fields[0].as_str();
        let err = cra_code_to_jurisdiction(code).expect_err(code);
        let msg = err.to_string();
        assert!(
            msg.contains(code) && msg.contains("unmapped"),
            "unmapped {code} must be a hard error naming the code, got {msg}"
        );
    }
}

/// Test 4 — special tokens map to BPA variants, not zero and not a parse failure.
#[test]
fn special_tokens_map_to_basic_personal_amount_variants() {
    assert_eq!(special_token("BPAF"), Some(SpecialToken::Bpaf));
    assert_eq!(special_token("BPAMB"), Some(SpecialToken::Bpamb));
    assert_eq!(special_token("BPAYT"), Some(SpecialToken::Bpayt));
    assert_eq!(
        special_token("No claim amount"),
        Some(SpecialToken::NoClaimAmount)
    );

    let text = fixture_text("tokens.csv");
    let other = parse_other_amounts("tokens.csv", &text).expect("tokens fixture");
    assert!(matches!(
        other["FED"].basic,
        Some(BasicCell::Token(SpecialToken::Bpaf))
    ));
    assert!(matches!(
        other["MB"].basic,
        Some(BasicCell::Token(SpecialToken::Bpamb))
    ));
    assert!(matches!(
        other["YT"].basic,
        Some(BasicCell::Token(SpecialToken::Bpayt))
    ));
    assert!(matches!(
        other["QC"].basic,
        Some(BasicCell::Token(SpecialToken::NoClaimAmount))
    ));
    assert!(matches!(
        other["ON"].basic,
        Some(BasicCell::Amount(ref a)) if a == "12989.00"
    ));

    let dummy = [netpay_ingest::Bracket {
        threshold: "0".into(),
        rate: "0.1400".into(),
        constant: "0.00".into(),
        ..Default::default()
    }];
    let mut bundle = empty_bundle();
    bundle.other = other;
    bundle
        .claim_code_1_tc
        .insert("FED".into(), "16452.00".into());
    bundle
        .claim_code_1_tc
        .insert("MB".into(), "15780.00".into());
    bundle.supplement = json!({
        "FED": { "bpa_min": "14829.00" }
    });
    // BPAF needs 5 brackets for phaseout indices 3 and 4.
    let fed_brackets = vec![
        dummy[0].clone(),
        netpay_ingest::Bracket {
            threshold: "58523.00".into(),
            rate: "0.2050".into(),
            constant: "3804.00".into(),
            ..Default::default()
        },
        netpay_ingest::Bracket {
            threshold: "117045.00".into(),
            rate: "0.2600".into(),
            constant: "10241.00".into(),
            ..Default::default()
        },
        netpay_ingest::Bracket {
            threshold: "181440.00".into(),
            rate: "0.2900".into(),
            constant: "15685.00".into(),
            ..Default::default()
        },
        netpay_ingest::Bracket {
            threshold: "258482.00".into(),
            rate: "0.3300".into(),
            constant: "26024.00".into(),
            ..Default::default()
        },
    ];

    let fed = netpay_ingest::jurisdiction_json("FED", &fed_brackets, &bundle).unwrap();
    let yt = netpay_ingest::jurisdiction_json("YT", &dummy, &bundle).unwrap();
    let qc = netpay_ingest::jurisdiction_json("QC", &dummy, &bundle).unwrap();
    let mb = netpay_ingest::jurisdiction_json("MB", &dummy, &bundle).unwrap();

    let fed_bpa: BasicPersonalAmount =
        serde_json::from_value(fed["basic_personal_amount"].clone()).unwrap();
    let yt_bpa: BasicPersonalAmount =
        serde_json::from_value(yt["basic_personal_amount"].clone()).unwrap();
    let qc_bpa: BasicPersonalAmount =
        serde_json::from_value(qc["basic_personal_amount"].clone()).unwrap();
    let mb_bpa: BasicPersonalAmount =
        serde_json::from_value(mb["basic_personal_amount"].clone()).unwrap();

    assert!(
        matches!(fed_bpa, BasicPersonalAmount::Dynamic { .. }),
        "BPAF → Dynamic, got {fed_bpa:?}"
    );
    assert!(
        matches!(mb_bpa, BasicPersonalAmount::Dynamic { .. }),
        "BPAMB → Dynamic, got {mb_bpa:?}"
    );
    assert!(
        matches!(yt_bpa, BasicPersonalAmount::SameAsFederal),
        "BPAYT → SameAsFederal, got {yt_bpa:?}"
    );
    assert!(
        matches!(qc_bpa, BasicPersonalAmount::NotApplicable),
        "No claim amount → NotApplicable, got {qc_bpa:?}"
    );
}

/// Test 5 — three lines per province. A truncated group (two lines) is a
/// hard error naming the jurisdiction.
#[test]
fn truncated_bracket_group_names_the_jurisdiction() {
    let text = fixture_text("brackets-truncated.csv");
    let err = parse_bracket_table("brackets-truncated.csv", &text)
        .expect_err("two-line ON group must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("ON"),
        "error must name the truncated jurisdiction, got {msg}"
    );
    assert!(
        msg.contains("truncated") || msg.contains("expected 3"),
        "error must say the group was truncated, got {msg}"
    );
}

/// Test 6 — ingest the archived January 2026 bundle; federal and Ontario JSON
/// match the M1 hand-authored files. `RuleSet::from_json` accepts the output.
#[test]
fn january_2026_federal_and_ontario_match_hand_authored() {
    let archive = workspace_data("sources/t4127-jan-2026");
    let expected_dir = workspace_data("rules/2026-01-01");
    let out = unique_temp("roundtrip");
    let report = ingest_archive(&archive, &out).unwrap_or_else(|e| panic!("ingest: {e}"));
    assert_eq!(report.effective_from, "2026-01-01");

    let got_fed: Value = read_json(&out.join("federal.json"));
    let got_on: Value = read_json(&out.join("on.json"));
    let expected_fed: Value = read_json(&expected_dir.join("federal.json"));
    let expected_on: Value = read_json(&expected_dir.join("on.json"));
    assert_eq!(
        got_fed, expected_fed,
        "federal.json ingest vs M1 hand-authored"
    );
    assert_eq!(got_on, expected_on, "on.json ingest vs M1 hand-authored");

    let assembled = assemble_ruleset(&out, &got_fed, &got_on);
    RuleSet::from_json(&assembled).expect("RuleSet::from_json must accept ingest output unchanged");

    let manifest: Value = read_json(&out.join("manifest.json"));
    let source = &manifest["sources"][0];
    assert_eq!(
        source["retrieved_at"], "2026-09-12T03:58:28Z",
        "retrieved_at comes from archive.json, not a clock"
    );
    assert!(source["source_url"]
        .as_str()
        .unwrap()
        .starts_with("https://"));
    assert_eq!(source["tool_version"], netpay_ingest::tool_version());
    assert!(!source["ingest_git_sha"].as_str().unwrap().is_empty());
    assert!(
        source["archive_files"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "manifest lists sha256 of each archived source file"
    );
    let _ = fs::remove_dir_all(&out);
}

/// Test 7 — a corrupted K digit fails the test-28 identity canary when loaded
/// through netpay-core. Ingest itself does not parse numbers.
#[test]
fn corrupted_k_digit_fails_identity_canary() {
    let ok_text = fixture_text("brackets-federal-ok.csv");
    let ok = parse_bracket_table("brackets-federal-ok.csv", &ok_text).unwrap();
    let ok_set = ruleset_from_brackets(&ok["FED"]);
    let ok_loaded = RuleSet::from_json(&ok_set).expect("good K must load");
    assert!(
        k_identity_error(ok_loaded.jurisdictions.get(&fed_code()).unwrap()).is_none(),
        "hand-check fixture must satisfy the test-28 K canary"
    );

    let bad_text = fixture_text("corrupted-k-digit.csv");
    let bad = parse_bracket_table("corrupted-k-digit.csv", &bad_text).unwrap();
    assert_eq!(
        bad["FED"][1].constant, "3805.00",
        "fixture must change K 3804 → 3805"
    );
    let bad_set = ruleset_from_brackets(&bad["FED"]);
    let loaded = RuleSet::from_json(&bad_set).expect("schema still accepts the corrupted K");
    let err = k_identity_error(loaded.jurisdictions.get(&fed_code()).unwrap());
    assert!(
        err.is_some(),
        "corrupted K must fail the test-28 identity canary"
    );
}

fn empty_bundle() -> netpay_ingest::ParsedBundle {
    netpay_ingest::ParsedBundle {
        brackets: BTreeMap::new(),
        other: BTreeMap::new(),
        claim_code_1_tc: BTreeMap::new(),
        claim_tables: BTreeMap::new(),
        cpp: json!({}),
        ei: json!({}),
        qpip: json!({}),
        supplement: json!({}),
    }
}

fn unique_temp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("netpay-ingest-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn fed_code() -> JurisdictionCode {
    JurisdictionCode("FED".into())
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("{}: {e}", path.display());
    }))
    .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn assemble_ruleset(out: &Path, fed: &Value, on: &Value) -> String {
    let manifest: Value = read_json(&out.join("manifest.json"));
    let src = &manifest["sources"][0];
    serde_json::to_string(&json!({
        "rule_set_version": manifest["rule_set_version"],
        "effective_from": manifest["effective_from"],
        "effective_to": manifest["effective_to"],
        "published_by": manifest["published_by"],
        "source_document": src["source_document"],
        "source_url": src["source_url"],
        "retrieved_at": src["retrieved_at"],
        "source_sha256": src["source_sha256"],
        "jurisdictions": {
            "FED": fed,
            "ON": on
        },
        "cpp": read_json(&out.join("cpp.json")),
        "ei": read_json(&out.join("ei.json")),
        "qpip": read_json(&out.join("qpip.json"))
    }))
    .unwrap()
}

const STUB_CPP: &str = r#"{
    "ympe": "74600.00",
    "yampe": "85000.00",
    "basic_exemption": "3500.00",
    "total_rate": "0.0595",
    "total_max": "4230.45",
    "base_rate": "0.0495",
    "base_max": "3519.45",
    "first_additional_rate": "0.0100",
    "first_additional_max": "711.00",
    "second_additional_rate": "0.0400",
    "second_additional_max": "416.00"
}"#;

fn ruleset_from_brackets(brackets: &[netpay_ingest::Bracket]) -> String {
    let lowest = &brackets[0].rate;
    let j = json!({
        "brackets": brackets.iter().map(|b| json!({
            "threshold": b.threshold,
            "rate": b.rate,
            "constant": b.constant
        })).collect::<Vec<_>>(),
        "lowest_rate": lowest,
        "basic_personal_amount": { "type": "fixed", "amount": "16452.00" }
    });
    serde_json::to_string(&json!({
        "rule_set_version": "2026-01-01",
        "effective_from": "2026-01-01",
        "effective_to": null,
        "published_by": "CRA",
        "source_document": "fixture",
        "source_url": "https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas.html",
        "retrieved_at": "2026-01-15T00:00:00Z",
        "source_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "jurisdictions": { "FED": j },
        "cpp": serde_json::from_str::<Value>(STUB_CPP).unwrap(),
        "ei": {
            "max_insurable": "68900.00",
            "employee_rate": "0.0163",
            "employee_max": "1123.07",
            "employer_rate": "0.02282",
            "employer_max": "1572.30"
        },
        "qpip": {
            "max_insurable": "103000.00",
            "employee_rate": "0.00430",
            "employee_max": "442.90",
            "employer_rate": "0.00602",
            "employer_max": "620.06"
        }
    }))
    .unwrap()
}

fn k_identity_error(jurisdiction: &Jurisdiction) -> Option<String> {
    let OptionScoped::Both(brackets) = &jurisdiction.brackets else {
        return Some("expected Both brackets".into());
    };
    if !brackets[0].constant.is_zero() {
        return Some("first constant must be 0".into());
    }
    let one = Money::parse("1").unwrap();
    let mut cumulative = Money::ZERO;
    for i in 1..brackets.len() {
        let t = brackets[i].threshold;
        let high = t.checked_mul_rate(brackets[i].rate).unwrap();
        let low = t.checked_mul_rate(brackets[i - 1].rate).unwrap();
        cumulative = cumulative
            .checked_add(high.checked_sub(low).unwrap())
            .unwrap();
        let expected = round_claim_to_dollar(cumulative.checked_div(one).unwrap());
        if brackets[i].constant != expected {
            return Some(format!(
                "bracket {i}: published {} identity {}",
                brackets[i].constant, expected
            ));
        }
    }
    None
}
