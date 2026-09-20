//! Compile-time embedding of hand-authored rule JSON (spec §9.1).
//!
//! `takehome-core` stays IO-free: every byte arrives through `include_str!`.
//! Provenance lives in `manifest.json` (source URL, retrieved_at, sha256).

use crate::rules::registry::{Registry, RuleError};
use crate::rules::schema::{
    CppParams, EiParams, Jurisdiction, JurisdictionCode, LoadError, QpipParams, RuleSet,
    RuleSetStatus,
};
use crate::rules::signature::{verify_rules_file, SignatureError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;
use thiserror::Error;

const MANIFEST_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/manifest.json"
));
const FEDERAL_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/federal.json"
));
const AB_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/ab.json"
));
const BC_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/bc.json"
));
const MB_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/mb.json"
));
const NB_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/nb.json"
));
const NL_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/nl.json"
));
const NS_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/ns.json"
));
const NT_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/nt.json"
));
const NU_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/nu.json"
));
const ON_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/on.json"
));
const PE_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/pe.json"
));
const SK_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/sk.json"
));
const YT_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/yt.json"
));
const CPP_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/cpp.json"
));
const EI_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/ei.json"
));
const QPIP_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-01-01/qpip.json"
));

const MANIFEST_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/manifest.json"
));
const FEDERAL_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/federal.json"
));
const AB_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/ab.json"
));
const BC_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/bc.json"
));
const MB_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/mb.json"
));
const NB_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/nb.json"
));
const NL_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/nl.json"
));
const NS_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/ns.json"
));
const NT_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/nt.json"
));
const NU_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/nu.json"
));
const ON_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/on.json"
));
const PE_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/pe.json"
));
const SK_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/sk.json"
));
const YT_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/yt.json"
));
const CPP_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/cpp.json"
));
const EI_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/ei.json"
));
const QPIP_2026_07_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2026-07-01/qpip.json"
));

const MANIFEST_2027_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2027-01-01/manifest.json"
));
const BC_2027_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2027-01-01/bc.json"
));
const NL_2027_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2027-01-01/nl.json"
));
const PE_2027_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2027-01-01/pe.json"
));
const CPP_2027_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/2027-01-01/cpp.json"
));

/// Lazily assembled registry of every embedded rule set.
pub static EMBEDDED_REGISTRY: LazyLock<Registry> =
    LazyLock::new(|| load_embedded_registry().expect("embedded rule sets must load"));

/// Catalog of embedded T4127 editions (spec §9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleSetListing {
    pub rule_set_versions: Vec<RuleSetVersionStatus>,
}

/// One embedded rule-set edition and its half-open coverage interval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleSetVersionStatus {
    pub version: String,
    pub effective_from: crate::rules::schema::CalendarDate,
    pub effective_to: Option<crate::rules::schema::CalendarDate>,
    /// Spec open question 5: enacted T4127 edition or labelled preview.
    pub status: RuleSetStatus,
}

/// Embedded editions in coverage order (spec §9.1).
pub fn list_embedded_rule_set_versions() -> RuleSetListing {
    RuleSetListing {
        rule_set_versions: EMBEDDED_REGISTRY
            .versions()
            .into_iter()
            .map(|(version, from, to)| RuleSetVersionStatus {
                version: version.to_string(),
                effective_from: from,
                effective_to: to,
                status: EMBEDDED_REGISTRY
                    .get(version)
                    .map(|set| set.status)
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

/// Failure assembling embedded rule JSON into a [`Registry`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EmbedError {
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("jurisdiction {code}: {source}")]
    Jurisdiction {
        code: String,
        #[source]
        source: LoadError,
    },
    #[error("cpp: {0}")]
    Cpp(String),
    #[error("ei: {0}")]
    Ei(String),
    #[error("qpip: {0}")]
    Qpip(String),
    #[error("signature: {0}")]
    Signature(#[from] SignatureError),
    #[error("ruleset validate: {0}")]
    Validate(#[from] LoadError),
    #[error("registry: {0}")]
    Registry(#[from] RuleError),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    rule_set_version: String,
    effective_from: crate::rules::schema::CalendarDate,
    #[serde(default)]
    effective_to: Option<crate::rules::schema::CalendarDate>,
    published_by: String,
    sources: Vec<ManifestSource>,
    #[serde(default)]
    #[allow(dead_code)]
    edition_kind: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    delta_marker: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    jurisdiction_origin: BTreeMap<String, String>,
    #[serde(default)]
    status: RuleSetStatus,
    #[serde(default)]
    announcement_source: Option<String>,
    #[serde(default)]
    announcement_date: Option<crate::rules::schema::CalendarDate>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // provenance fields are accepted so ingest output can load later
struct ManifestSource {
    source_document: String,
    source_url: String,
    retrieved_at: String,
    #[serde(default)]
    scalars_verified_at: Option<String>,
    source_sha256: String,
    files: Vec<String>,
    /// Present when this rule directory was emitted by `takehome-ingest`.
    #[serde(default)]
    tool_version: Option<String>,
    #[serde(default)]
    ingest_git_sha: Option<String>,
    #[serde(default)]
    archive_files: Option<Vec<ArchiveFileHash>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // deserialized for ingest provenance, not read by the embed path
struct ArchiveFileHash {
    name: String,
    sha256: String,
}

fn load_jurisdiction(json: &str, code: &str) -> Result<Jurisdiction, EmbedError> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|err| EmbedError::Jurisdiction {
        code: code.to_string(),
        source: LoadError {
            message: err.to_string(),
        },
    })
}

fn load_component<T: serde::de::DeserializeOwned>(
    json: &str,
    map_err: fn(String) -> EmbedError,
) -> Result<T, EmbedError> {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|err| map_err(err.to_string()))
}

/// Build the 2026-01-01 [`RuleSet`] from embedded JSON fragments.
pub fn load_ruleset_2026_01_01() -> Result<RuleSet, EmbedError> {
    assemble_ruleset(
        "2026-01-01/manifest.json",
        MANIFEST_2026_01_01,
        [
            ("FED", "2026-01-01/federal.json", FEDERAL_2026_01_01),
            ("AB", "2026-01-01/ab.json", AB_2026_01_01),
            ("BC", "2026-01-01/bc.json", BC_2026_01_01),
            ("MB", "2026-01-01/mb.json", MB_2026_01_01),
            ("NB", "2026-01-01/nb.json", NB_2026_01_01),
            ("NL", "2026-01-01/nl.json", NL_2026_01_01),
            ("NS", "2026-01-01/ns.json", NS_2026_01_01),
            ("NT", "2026-01-01/nt.json", NT_2026_01_01),
            ("NU", "2026-01-01/nu.json", NU_2026_01_01),
            ("ON", "2026-01-01/on.json", ON_2026_01_01),
            ("PE", "2026-01-01/pe.json", PE_2026_01_01),
            ("SK", "2026-01-01/sk.json", SK_2026_01_01),
            ("YT", "2026-01-01/yt.json", YT_2026_01_01),
        ],
        ("2026-01-01/cpp.json", CPP_2026_01_01),
        ("2026-01-01/ei.json", EI_2026_01_01),
        ("2026-01-01/qpip.json", QPIP_2026_01_01),
    )
}

/// Build the 2026-07-01 [`RuleSet`] from the July delta overlaid on January.
pub fn load_ruleset_2026_07_01() -> Result<RuleSet, EmbedError> {
    assemble_ruleset(
        "2026-07-01/manifest.json",
        MANIFEST_2026_07_01,
        [
            ("FED", "2026-07-01/federal.json", FEDERAL_2026_07_01),
            ("AB", "2026-07-01/ab.json", AB_2026_07_01),
            ("BC", "2026-07-01/bc.json", BC_2026_07_01),
            ("MB", "2026-07-01/mb.json", MB_2026_07_01),
            ("NB", "2026-07-01/nb.json", NB_2026_07_01),
            ("NL", "2026-07-01/nl.json", NL_2026_07_01),
            ("NS", "2026-07-01/ns.json", NS_2026_07_01),
            ("NT", "2026-07-01/nt.json", NT_2026_07_01),
            ("NU", "2026-07-01/nu.json", NU_2026_07_01),
            ("ON", "2026-07-01/on.json", ON_2026_07_01),
            ("PE", "2026-07-01/pe.json", PE_2026_07_01),
            ("SK", "2026-07-01/sk.json", SK_2026_07_01),
            ("YT", "2026-07-01/yt.json", YT_2026_07_01),
        ],
        ("2026-07-01/cpp.json", CPP_2026_07_01),
        ("2026-07-01/ei.json", EI_2026_07_01),
        ("2026-07-01/qpip.json", QPIP_2026_07_01),
    )
}

/// Build the 2027-01-01 PREVIEW [`RuleSet`] (spec open question 5).
///
/// Not a CRA T4127 edition. CPP rates from the Spring Economic Update 2026;
/// YMPE held at 2026 published dollars; BC indexation paused at 2026 levels.
pub fn load_ruleset_2027_01_01() -> Result<RuleSet, EmbedError> {
    assemble_ruleset(
        "2027-01-01/manifest.json",
        MANIFEST_2027_01_01,
        [
            ("FED", "2026-07-01/federal.json", FEDERAL_2026_07_01),
            ("AB", "2026-07-01/ab.json", AB_2026_07_01),
            ("BC", "2027-01-01/bc.json", BC_2027_01_01),
            ("MB", "2026-07-01/mb.json", MB_2026_07_01),
            ("NB", "2026-07-01/nb.json", NB_2026_07_01),
            ("NL", "2027-01-01/nl.json", NL_2027_01_01),
            ("NS", "2026-07-01/ns.json", NS_2026_07_01),
            ("NT", "2026-07-01/nt.json", NT_2026_07_01),
            ("NU", "2026-07-01/nu.json", NU_2026_07_01),
            ("ON", "2026-07-01/on.json", ON_2026_07_01),
            ("PE", "2027-01-01/pe.json", PE_2027_01_01),
            ("SK", "2026-07-01/sk.json", SK_2026_07_01),
            ("YT", "2026-07-01/yt.json", YT_2026_07_01),
        ],
        ("2027-01-01/cpp.json", CPP_2027_01_01),
        ("2026-07-01/ei.json", EI_2026_07_01),
        ("2026-07-01/qpip.json", QPIP_2026_07_01),
    )
}

fn assemble_ruleset(
    manifest_path: &str,
    manifest_json: &str,
    jurisdiction_files: [(&str, &str, &str); 13],
    cpp: (&str, &str),
    ei: (&str, &str),
    qpip: (&str, &str),
) -> Result<RuleSet, EmbedError> {
    verify_rules_file(manifest_path, manifest_json.as_bytes())?;
    for (_, path, json) in jurisdiction_files {
        verify_rules_file(path, json.as_bytes())?;
    }
    verify_rules_file(cpp.0, cpp.1.as_bytes())?;
    verify_rules_file(ei.0, ei.1.as_bytes())?;
    verify_rules_file(qpip.0, qpip.1.as_bytes())?;

    let mut deserializer = serde_json::Deserializer::from_str(manifest_json);
    let manifest: Manifest = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|err| EmbedError::Manifest(err.to_string()))?;
    let source = manifest
        .sources
        .first()
        .ok_or_else(|| EmbedError::Manifest("manifest has no sources".to_string()))?;
    for required in [
        "federal.json",
        "ab.json",
        "bc.json",
        "mb.json",
        "nb.json",
        "nl.json",
        "ns.json",
        "nt.json",
        "nu.json",
        "on.json",
        "pe.json",
        "sk.json",
        "yt.json",
        "cpp.json",
        "ei.json",
        "qpip.json",
    ] {
        if !source.files.iter().any(|f| f == required) {
            return Err(EmbedError::Manifest(format!(
                "manifest sources[0].files missing {required}"
            )));
        }
    }

    let mut jurisdictions = BTreeMap::new();
    for (code, _, json) in jurisdiction_files {
        jurisdictions.insert(
            JurisdictionCode(code.to_string()),
            load_jurisdiction(json, code)?,
        );
    }

    let cpp: CppParams = load_component(cpp.1, EmbedError::Cpp)?;
    let ei: EiParams = load_component(ei.1, EmbedError::Ei)?;
    let qpip: QpipParams = load_component(qpip.1, EmbedError::Qpip)?;

    let set = RuleSet {
        rule_set_version: manifest.rule_set_version,
        effective_from: manifest.effective_from,
        effective_to: manifest.effective_to,
        published_by: manifest.published_by,
        source_document: source.source_document.clone(),
        source_url: source.source_url.clone(),
        retrieved_at: source.retrieved_at.clone(),
        source_sha256: source.source_sha256.clone(),
        status: manifest.status,
        announcement_source: manifest.announcement_source,
        announcement_date: manifest.announcement_date,
        jurisdictions,
        cpp,
        ei,
        qpip,
    };
    set.validate_assembled()?;
    Ok(set)
}

/// Every embedded rule set, sorted and coverage-checked.
pub fn load_embedded_registry() -> Result<Registry, EmbedError> {
    Registry::load(vec![
        load_ruleset_2026_01_01()?,
        load_ruleset_2026_07_01()?,
        load_ruleset_2027_01_01()?,
    ])
    .map_err(EmbedError::from)
}

#[cfg(test)]
mod tests {
    use super::{
        load_ruleset_2026_01_01, load_ruleset_2026_07_01, load_ruleset_2027_01_01,
        EMBEDDED_REGISTRY,
    };
    use crate::decimal::{Money, Rate};
    use crate::formulas::bpa::resolve_basic_personal_amount;
    use crate::formulas::indexing::{index_claim_amount, IndexError};
    use crate::formulas::k_identity::{k_exact_constants, k_residual};
    use crate::rounding::{round_claim_to_dollar, round_contribution_to_cent};
    use crate::rules::schema::{
        BasicPersonalAmount, CalculationOption, CppParams, EiParams, Jurisdiction,
        JurisdictionCode, OptionScoped, QpipParams, RuleSet,
    };
    use std::path::PathBuf;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

    fn j<'a>(set: &'a RuleSet, code: &str) -> &'a Jurisdiction {
        set.jurisdictions
            .get(&JurisdictionCode(code.to_string()))
            .unwrap_or_else(|| panic!("{code} must load"))
    }

    const TABLE_8_1_CODES: &[&str] = &[
        "FED", "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "SK", "YT",
    ];

    /// 21. Every embedded rule set loads without error.
    #[test]
    fn every_embedded_rule_set_loads_without_error() {
        let set = load_ruleset_2026_01_01().expect("2026-01-01 must load");
        assert_eq!(set.rule_set_version, "2026-01-01");
        let _ = &*EMBEDDED_REGISTRY;
        assert_eq!(
            EMBEDDED_REGISTRY
                .resolve("2026-01-01".parse().unwrap())
                .unwrap()
                .rule_set_version,
            "2026-01-01"
        );
        assert_eq!(set.jurisdictions.len(), 13);
        for code in TABLE_8_1_CODES {
            assert!(
                set.jurisdictions
                    .contains_key(&JurisdictionCode(code.to_string())),
                "missing {code}"
            );
        }
        let july = load_ruleset_2026_07_01().expect("2026-07-01 must load");
        assert_eq!(july.rule_set_version, "2026-07-01");
        assert_eq!(
            EMBEDDED_REGISTRY
                .resolve("2026-07-01".parse().unwrap())
                .unwrap()
                .rule_set_version,
            "2026-07-01"
        );
        let preview = load_ruleset_2027_01_01().expect("2027-01-01 preview must load");
        assert_eq!(preview.rule_set_version, "2027-01-01");
        assert_eq!(
            preview.status,
            crate::rules::schema::RuleSetStatus::Proposed
        );
        assert_eq!(
            EMBEDDED_REGISTRY
                .resolve("2027-01-15".parse().unwrap())
                .unwrap()
                .rule_set_version,
            "2027-01-01"
        );
    }

    /// 22. Federal brackets: 5 rows, thresholds / rates / constants from Table 8.1.
    #[test]
    fn federal_brackets_match_t4127_table_8_1() {
        let set = load_ruleset_2026_01_01().unwrap();
        let fed = set
            .jurisdictions
            .get(&JurisdictionCode("FED".to_string()))
            .expect("FED");
        let brackets = fed.brackets.get(CalculationOption::Option1);
        assert_eq!(brackets.len(), 5);
        let expected = [
            ("0", "0.1400", "0.00"),
            ("58523.00", "0.2050", "3804.00"),
            ("117045.00", "0.2600", "10241.00"),
            ("181440.00", "0.2900", "15685.00"),
            ("258482.00", "0.3300", "26024.00"),
        ];
        for (bracket, (t, r, k)) in brackets.iter().zip(expected) {
            assert_eq!(bracket.threshold, money(t));
            assert_eq!(bracket.rate, rate(r));
            assert_eq!(bracket.constant, money(k));
        }
    }

    /// 23. Ontario brackets are exactly 5; first threshold 0 rate 0.0505 constant 0.
    #[test]
    fn ontario_brackets_five_first_is_0_0505() {
        let set = load_ruleset_2026_01_01().unwrap();
        let on = set
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .expect("ON");
        let brackets = on.brackets.get(CalculationOption::Option1);
        assert_eq!(brackets.len(), 5);
        assert_eq!(brackets[0].threshold, money("0"));
        assert_eq!(brackets[0].rate, rate("0.0505"));
        assert_eq!(brackets[0].constant, money("0.00"));
    }

    /// 24. CPP parameters from T4127 Tables 8.3–8.6.
    #[test]
    fn cpp_params_match_t4127_tables() {
        let cpp = &load_ruleset_2026_01_01().unwrap().cpp;
        assert_eq!(cpp.ympe, money("74600.00"));
        assert_eq!(cpp.yampe, money("85000.00"));
        assert_eq!(cpp.basic_exemption, money("3500.00"));
        assert_eq!(cpp.total_rate, rate("0.0595"));
        assert_eq!(cpp.total_max, money("4230.45"));
        assert_eq!(cpp.base_rate, rate("0.0495"));
        assert_eq!(cpp.base_max, money("3519.45"));
        assert_eq!(cpp.first_additional_rate, rate("0.0100"));
        assert_eq!(cpp.first_additional_max, money("711.00"));
        assert_eq!(cpp.second_additional_rate, rate("0.0400"));
        assert_eq!(cpp.second_additional_max, money("416.00"));
    }

    /// 25. EI parameters from T4127 Table 8.7 (Canada except QC).
    #[test]
    fn ei_params_match_t4127_table_8_7() {
        let ei = &load_ruleset_2026_01_01().unwrap().ei;
        assert_eq!(ei.max_insurable, money("68900.00"));
        assert_eq!(ei.employee_rate, rate("0.0163"));
        assert_eq!(ei.employee_max, money("1123.07"));
        assert_eq!(ei.employer_rate, rate("0.02282"));
        assert_eq!(ei.employer_max, money("1572.30"));
    }

    /// 26. BPAF is Dynamic with max 16452, min 14829, phaseout 181440 → 258482.
    #[test]
    fn bpaf_is_dynamic_with_phaseout() {
        let set = load_ruleset_2026_01_01().unwrap();
        let fed = set
            .jurisdictions
            .get(&JurisdictionCode("FED".to_string()))
            .unwrap();
        match fed.basic_personal_amount.get(CalculationOption::Option1) {
            BasicPersonalAmount::Dynamic {
                max,
                min,
                phaseout_start,
                phaseout_end,
                ..
            } => {
                assert_eq!(*max, money("16452.00"));
                assert_eq!(*min, money("14829.00"));
                assert_eq!(*phaseout_start, money("181440.00"));
                assert_eq!(*phaseout_end, money("258482.00"));
            }
            other => panic!("expected Dynamic BPAF, got {other:?}"),
        }
    }

    /// 27. Ontario has 2 surtax tiers and 6 health premium tiers.
    #[test]
    fn ontario_surtax_and_health_premium_tier_counts() {
        let set = load_ruleset_2026_01_01().unwrap();
        let on = set
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .unwrap();
        assert_eq!(on.surtax.as_ref().map(|t| t.len()), Some(2));
        assert_eq!(on.health_premium.as_ref().map(|t| t.len()), Some(6));
    }

    // Test 28 — bracket transcription canary: for every bracket i>0,
    // constant[i] == round_claim_to_dollar(Σ threshold[j]*(rate[j]-rate[j-1])).
    //
    // Covered: every Table 8.1 threshold, rate, and constant in all thirteen
    // jurisdictions (FED + 12 provinces/territories; QC is not in Table 8.1).
    // A mistype in any of those moves the residual and fails.
    //
    // Not covered: scalars that do not participate in the bracket identity
    // (BPAF min/max/phaseout, CEA, LCF, Ontario S2/Y/V1/V2, CPP, EI, QPIP).
    // Those are ticked against T4127 in
    // `non_bracket_scalars_match_t4127_2026_01_01` and algebraically in 28b.
    #[test]
    fn bracket_k_constants_match_cumulative_rate_delta_identity() {
        for set in [
            load_ruleset_2026_01_01().unwrap(),
            load_ruleset_2026_07_01().unwrap(),
        ] {
            for (code, jurisdiction) in &set.jurisdictions {
                match &jurisdiction.brackets {
                    OptionScoped::Both(brackets) => {
                        assert_k_identity(
                            &code.0,
                            &format!("{}/both", set.rule_set_version),
                            brackets,
                        );
                    }
                    OptionScoped::PerOption { option1, option2 } => {
                        assert_k_identity(
                            &code.0,
                            &format!("{}/option1", set.rule_set_version),
                            option1,
                        );
                        assert_k_identity(
                            &code.0,
                            &format!("{}/option2", set.rule_set_version),
                            option2,
                        );
                    }
                }
            }
        }
    }

    fn assert_k_identity(code: &str, scope: &str, brackets: &[crate::rules::schema::Bracket]) {
        assert!(
            !brackets.is_empty() && brackets[0].constant.is_zero(),
            "{code}/{scope}: first constant must be 0"
        );
        let one = Money::parse("1").unwrap();
        let mut cumulative = Money::ZERO;
        for i in 1..brackets.len() {
            let t = brackets[i].threshold;
            let high = t
                .checked_mul_rate(brackets[i].rate)
                .expect("threshold * rate");
            let low = t
                .checked_mul_rate(brackets[i - 1].rate)
                .expect("threshold * prev rate");
            let delta = high.checked_sub(low).expect("rate-delta product");
            cumulative = cumulative.checked_add(delta).expect("cum add");
            let as_ratio = cumulative.checked_div(one).expect("money as ratio");
            let expected = round_claim_to_dollar(as_ratio);
            assert_eq!(
                brackets[i].constant, expected,
                "{code}/{scope} bracket {i}: K canary failed (published {}, identity {})",
                brackets[i].constant, expected
            );
        }
    }

    fn unit() -> Money {
        money("1")
    }

    fn rate_as_money(value: Rate) -> Money {
        unit().checked_mul_rate(value).unwrap()
    }

    fn rounded_product(amount: Money, value: Rate) -> Money {
        round_contribution_to_cent(
            amount
                .checked_mul_rate(value)
                .unwrap()
                .checked_div(unit())
                .unwrap(),
        )
    }

    fn assert_cpp_identities(label: &str, cpp: &CppParams) {
        assert_eq!(
            rate_as_money(cpp.base_rate)
                .checked_add(rate_as_money(cpp.first_additional_rate))
                .unwrap(),
            rate_as_money(cpp.total_rate),
            "{label}: base_rate + first_additional_rate != total_rate"
        );
        assert_eq!(
            cpp.base_max.checked_add(cpp.first_additional_max).unwrap(),
            cpp.total_max,
            "{label}: base_max + first_additional_max != total_max"
        );
        assert_eq!(
            cpp.ympe.checked_sub(cpp.basic_exemption).unwrap(),
            money("71100.00"),
            "{label}: YMPE − basic_exemption != YMCE 71100.00"
        );
        assert_eq!(
            rounded_product(
                cpp.yampe.checked_sub(cpp.ympe).unwrap(),
                cpp.second_additional_rate
            ),
            cpp.second_additional_max,
            "{label}: (YAMPE − YMPE) × second_additional_rate != second_additional_max"
        );
    }

    fn assert_premium_max(label: &str, max_insurable: Money, value: Rate, published_max: Money) {
        assert_eq!(
            rounded_product(max_insurable, value),
            published_max,
            "{label}: round(max_insurable × rate) != published max"
        );
    }

    fn assert_ei_identities(label: &str, ei: &EiParams) {
        assert_premium_max(
            &format!("{label} employee"),
            ei.max_insurable,
            ei.employee_rate,
            ei.employee_max,
        );
        assert_premium_max(
            &format!("{label} employer"),
            ei.max_insurable,
            ei.employer_rate,
            ei.employer_max,
        );
    }

    fn assert_qpip_identities(qpip: &QpipParams) {
        assert_premium_max(
            "QPIP employee",
            qpip.max_insurable,
            qpip.employee_rate,
            qpip.employee_max,
        );
        assert_premium_max(
            "QPIP employer",
            qpip.max_insurable,
            qpip.employer_rate,
            qpip.employer_max,
        );
    }

    /// 28b. Cross-file algebraic identities the published tables must satisfy.
    /// Catches a mistyped maximum with no external oracle.
    #[test]
    fn contribution_table_identities_hold_for_cpp_ei_qpip() {
        for set in [
            load_ruleset_2026_01_01().unwrap(),
            load_ruleset_2026_07_01().unwrap(),
        ] {
            let label = set.rule_set_version.as_str();
            assert_cpp_identities(&format!("CPP Table 8.3–8.6 ({label})"), &set.cpp);
            assert_ei_identities(&format!("EI Table 8.7 Canada except QC ({label})"), &set.ei);
            assert_qpip_identities(&set.qpip);
        }

        // QPP / Quebec EI are not a calculated jurisdiction, but Table 8.3–8.7
        // QC rows must still satisfy the same identities.
        let mut qpp = load_ruleset_2026_01_01().unwrap().cpp.clone();
        qpp.total_rate = rate("0.0630");
        qpp.total_max = money("4479.30");
        qpp.base_rate = rate("0.0530");
        qpp.base_max = money("3768.30");
        assert_cpp_identities("QPP Table 8.3–8.6", &qpp);

        let ei_qc = EiParams {
            max_insurable: money("68900.00"),
            employee_rate: rate("0.0130"),
            employee_max: money("895.70"),
            employer_rate: rate("0.01820"),
            employer_max: money("1253.98"),
        };
        assert_ei_identities("EI Table 8.7 QC", &ei_qc);
    }

    /// Non-bracket scalars ticked against T4127 11 Jan 2026 / Chapter 8.
    /// Test 28 does not cover these; a wrong BPAF phaseout denominator is
    /// plausible everywhere and wrong only between $181,440 and $258,482.
    #[test]
    fn non_bracket_scalars_match_t4127_2026_01_01() {
        let set = load_ruleset_2026_01_01().unwrap();
        let fed = set
            .jurisdictions
            .get(&JurisdictionCode("FED".to_string()))
            .unwrap();
        match fed.basic_personal_amount.get(CalculationOption::Option1) {
            BasicPersonalAmount::Dynamic {
                max,
                min,
                phaseout_start,
                phaseout_end,
                ..
            } => {
                assert_eq!(*max, money("16452.00"));
                assert_eq!(*min, money("14829.00"));
                assert_eq!(*phaseout_start, money("181440.00"));
                assert_eq!(*phaseout_end, money("258482.00"));
                assert_eq!(max.checked_sub(*min).unwrap(), money("1623.00"));
                assert_eq!(
                    phaseout_end.checked_sub(*phaseout_start).unwrap(),
                    money("77042.00")
                );
            }
            other => panic!("expected Dynamic BPAF, got {other:?}"),
        }
        assert_eq!(fed.canada_employment_amount, Some(money("1501.00")));
        assert_eq!(fed.index_rate, Some(rate("0.020")));
        let lcf = fed.lcf.as_ref().expect("federal LCF");
        assert_eq!(lcf.rate, rate("0.150"));
        assert_eq!(lcf.max, money("750.00"));

        let on = set
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .unwrap();
        match on.basic_personal_amount.get(CalculationOption::Option1) {
            BasicPersonalAmount::Fixed { amount } => {
                assert_eq!(*amount, money("12989.00"));
            }
            other => panic!("expected fixed Ontario BPA, got {other:?}"),
        }
        assert_eq!(on.index_rate, Some(rate("0.019")));
        let reduction = on
            .tax_reduction
            .as_ref()
            .unwrap()
            .get(CalculationOption::Option1);
        assert_eq!(reduction.basic, money("300.00"));
        assert_eq!(reduction.dependant, money("554.00"));
        let surtax = on.surtax.as_ref().expect("Ontario surtax");
        assert_eq!(surtax[0].threshold, money("5818.00"));
        assert_eq!(surtax[0].rate, rate("0.20"));
        assert_eq!(surtax[1].threshold, money("7446.00"));
        assert_eq!(surtax[1].rate, rate("0.36"));
        let v2 = on.health_premium.as_ref().expect("Ontario health premium");
        let expected_v2 = [
            ("0.00", "0.00", "0.00", "0.00"),
            ("20000.00", "0.06", "0.00", "300.00"),
            ("36000.00", "0.06", "300.00", "450.00"),
            ("48000.00", "0.25", "450.00", "600.00"),
            ("72000.00", "0.25", "600.00", "750.00"),
            ("200000.00", "0.25", "750.00", "900.00"),
        ];
        assert_eq!(v2.len(), expected_v2.len());
        for (tier, (threshold, r, base, cap)) in v2.iter().zip(expected_v2) {
            assert_eq!(tier.threshold, money(threshold));
            assert_eq!(tier.rate, rate(r));
            assert_eq!(tier.base, money(base));
            assert_eq!(tier.cap, money(cap));
        }

        let cpp = &set.cpp;
        assert_eq!(cpp.ympe, money("74600.00"));
        assert_eq!(cpp.yampe, money("85000.00"));
        assert_eq!(cpp.basic_exemption, money("3500.00"));
        assert_eq!(cpp.total_rate, rate("0.0595"));
        assert_eq!(cpp.total_max, money("4230.45"));
        assert_eq!(cpp.base_rate, rate("0.0495"));
        assert_eq!(cpp.base_max, money("3519.45"));
        assert_eq!(cpp.first_additional_rate, rate("0.0100"));
        assert_eq!(cpp.first_additional_max, money("711.00"));
        assert_eq!(cpp.second_additional_rate, rate("0.0400"));
        assert_eq!(cpp.second_additional_max, money("416.00"));

        let ei = &set.ei;
        assert_eq!(ei.max_insurable, money("68900.00"));
        assert_eq!(ei.employee_rate, rate("0.0163"));
        assert_eq!(ei.employee_max, money("1123.07"));
        assert_eq!(ei.employer_rate, rate("0.02282"));
        assert_eq!(ei.employer_max, money("1572.30"));

        let qpip = &set.qpip;
        assert_eq!(qpip.max_insurable, money("103000.00"));
        assert_eq!(qpip.employee_rate, rate("0.00430"));
        assert_eq!(qpip.employee_max, money("442.90"));
        assert_eq!(qpip.employer_rate, rate("0.00602"));
        assert_eq!(qpip.employer_max, money("620.06"));
    }

    /// Test 8 — per-jurisdiction K residuals (`K_exact − K_published`) pinned
    /// so a later mistype in one province is visible without re-deriving.
    #[test]
    fn k_residuals_match_committed_fixture_for_all_thirteen() {
        let set = load_ruleset_2026_01_01().unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/vectors/k_residuals_2026_01_01.json"
        )))
        .expect("k residuals fixture");
        let expected = fixture.get("residuals").expect("fixture.residuals");
        let mut got = serde_json::Map::new();
        for code in TABLE_8_1_CODES {
            let brackets = j(&set, code).brackets.get(CalculationOption::Option1);
            let exact = k_exact_constants(brackets).unwrap();
            let residuals: Vec<serde_json::Value> = exact
                .iter()
                .zip(brackets.iter())
                .map(|(e, b)| {
                    let residual = k_residual(*e, b.constant).unwrap();
                    serde_json::Value::String(format!(
                        "{}",
                        residual.checked_div(money("1")).unwrap()
                    ))
                })
                .collect();
            got.insert((*code).to_string(), serde_json::Value::Array(residuals));
        }
        assert_eq!(
            serde_json::Value::Object(got),
            *expected,
            "K residuals moved; a Table 8.1 constant or rate is wrong"
        );
    }

    /// Test 10 — spec §6.8 / T4127 Table 8.2 basic personal amount and index rate.
    /// Thirteen hand-typed assertions, not generated from ingest output.
    #[test]
    fn spec_6_8_basic_personal_amount_and_index_rate() {
        let set = load_ruleset_2026_01_01().unwrap();
        let opt = CalculationOption::Option1;

        // FED — BPAF, index 2.0%
        match j(&set, "FED").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Dynamic { max, .. } => {
                assert_eq!(*max, money("16452.00"));
            }
            other => panic!("FED BPAF: {other:?}"),
        }
        assert_eq!(j(&set, "FED").index_rate, Some(rate("0.020")));

        // AB — $22,769, index 2.0%
        match j(&set, "AB").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("22769.00")),
            other => panic!("AB: {other:?}"),
        }
        assert_eq!(j(&set, "AB").index_rate, Some(rate("0.020")));

        // BC — $13,216, index 2.2%
        match j(&set, "BC").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("13216.00")),
            other => panic!("BC: {other:?}"),
        }
        assert_eq!(j(&set, "BC").index_rate, Some(rate("0.022")));

        // MB — BPAMB, index 2.1%
        match j(&set, "MB").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Dynamic { max, min, .. } => {
                assert_eq!(*max, money("15780.00"));
                assert_eq!(*min, money("0.00"));
            }
            other => panic!("MB BPAMB: {other:?}"),
        }
        assert_eq!(j(&set, "MB").index_rate, Some(rate("0.021")));

        // NB — $13,664, index 2.0%
        match j(&set, "NB").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("13664.00")),
            other => panic!("NB: {other:?}"),
        }
        assert_eq!(j(&set, "NB").index_rate, Some(rate("0.020")));

        // NL — $11,188, index 1.1%
        match j(&set, "NL").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("11188.00")),
            other => panic!("NL: {other:?}"),
        }
        assert_eq!(j(&set, "NL").index_rate, Some(rate("0.011")));

        // NS — $11,932, index 1.6%
        match j(&set, "NS").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("11932.00")),
            other => panic!("NS: {other:?}"),
        }
        assert_eq!(j(&set, "NS").index_rate, Some(rate("0.016")));

        // NT — $18,198, index 2.0%
        match j(&set, "NT").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("18198.00")),
            other => panic!("NT: {other:?}"),
        }
        assert_eq!(j(&set, "NT").index_rate, Some(rate("0.020")));

        // NU — $19,659, index 2.0%
        match j(&set, "NU").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("19659.00")),
            other => panic!("NU: {other:?}"),
        }
        assert_eq!(j(&set, "NU").index_rate, Some(rate("0.020")));

        // ON — $12,989, index 1.9%
        match j(&set, "ON").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("12989.00")),
            other => panic!("ON: {other:?}"),
        }
        assert_eq!(j(&set, "ON").index_rate, Some(rate("0.019")));

        // PE — $15,000, no indexing
        match j(&set, "PE").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("15000.00")),
            other => panic!("PE: {other:?}"),
        }
        assert_eq!(j(&set, "PE").index_rate, None);

        // SK — $20,381, index 2.0%
        match j(&set, "SK").basic_personal_amount.get(opt) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("20381.00")),
            other => panic!("SK: {other:?}"),
        }
        assert_eq!(j(&set, "SK").index_rate, Some(rate("0.020")));

        // YT — BPAYT (same as federal), index 2.0%
        assert!(matches!(
            j(&set, "YT").basic_personal_amount.get(opt),
            BasicPersonalAmount::SameAsFederal
        ));
        assert_eq!(j(&set, "YT").index_rate, Some(rate("0.020")));
    }

    /// Test 11 — PE `index_rate` is `None`, not 0.0. Indexing must error on `None`.
    #[test]
    fn pei_does_not_index_and_indexing_errors_on_none() {
        let set = load_ruleset_2026_01_01().unwrap();
        let pe = j(&set, "PE");
        assert_eq!(pe.index_rate, None);
        assert_ne!(
            pe.index_rate,
            Some(rate("0")),
            "PE must not store a zero index rate"
        );
        assert_ne!(pe.index_rate, Some(rate("0.0")));
        assert_ne!(pe.index_rate, Some(rate("0.000")));
        let err = index_claim_amount(money("15000.00"), pe.index_rate)
            .expect_err("indexing PE must not no-op");
        assert_eq!(err, IndexError::NotApplicable);
        let zero_would_noop =
            index_claim_amount(money("15000.00"), Some(rate("0"))).expect("zero rate applies");
        assert_eq!(zero_would_noop, money("15000.00"));
    }

    /// Test 12 — NS 2026 has a fixed BPA. BPANS clawback is a 2025 artifact.
    #[test]
    fn nova_scotia_2026_has_no_bpans_clawback() {
        let set = load_ruleset_2026_01_01().unwrap();
        match j(&set, "NS")
            .basic_personal_amount
            .get(CalculationOption::Option1)
        {
            BasicPersonalAmount::Fixed { amount } => {
                assert_eq!(*amount, money("11932.00"));
            }
            other => panic!("NS 2026 must not be a BPANS Dynamic clawback, got {other:?}"),
        }
        let raw = super::NS_2026_01_01;
        assert!(
            !raw.contains("phaseout") && !raw.contains("dynamic") && !raw.contains("BPANS"),
            "ingest must not emit a 2025 BPANS formula for 2026"
        );
    }

    /// Test 13 — YT BPAYT resolves to federal BPAF by reference; CEA matches federal.
    #[test]
    fn yukon_bpayt_is_federal_reference_and_cea_matches() {
        let set = load_ruleset_2026_01_01().unwrap();
        let fed = j(&set, "FED");
        let yt = j(&set, "YT");
        let opt = CalculationOption::Option1;
        let fed_bpa = fed.basic_personal_amount.get(opt);
        let yt_bpa = yt.basic_personal_amount.get(opt);
        assert!(
            matches!(yt_bpa, BasicPersonalAmount::SameAsFederal),
            "BPAYT must be a reference, not copied Dynamic constants"
        );
        let ni = money("220000.00");
        let from_fed = resolve_basic_personal_amount(ni, fed_bpa, None).unwrap();
        let from_yt = resolve_basic_personal_amount(ni, yt_bpa, Some(fed_bpa)).unwrap();
        assert_eq!(from_yt, from_fed);
        assert_eq!(from_yt, money("15639.68"));
        assert_eq!(yt.canada_employment_amount, fed.canada_employment_amount);
        assert_eq!(yt.canada_employment_amount, Some(money("1501.00")));
    }

    #[test]
    fn scalars_verified_at_is_recorded_on_the_manifest() {
        let manifest: super::Manifest =
            serde_json::from_str(super::MANIFEST_2026_01_01).expect("manifest");
        assert_eq!(
            manifest.sources[0].scalars_verified_at.as_deref(),
            Some("2026-09-12T02:50:00Z")
        );
    }

    fn per_option<T>(scoped: &OptionScoped<T>) -> (&T, &T) {
        match scoped {
            OptionScoped::PerOption { option1, option2 } => (option1, option2),
            OptionScoped::Both(_) => panic!("expected PerOption, got Both"),
        }
    }

    /// Test 15 — BC July is OptionScoped: option1 lowest 0.0614 prorated, option2 0.0560
    /// with no prorated flag. Brackets, KP, S2 (805 vs 690) and the tax-reduction
    /// upper threshold (44952) are PerOption, not Both.
    #[test]
    fn july_bc_is_option_scoped_not_both() {
        let july = load_ruleset_2026_07_01().unwrap();
        let bc = j(&july, "BC");
        let (b1, b2) = per_option(&bc.brackets);
        assert_eq!(b1[0].rate, rate("0.0614"));
        assert!(b1[0].prorated, "option1 lowest bracket is prorated");
        assert_eq!(b2[0].rate, rate("0.0560"));
        assert!(!b2[0].prorated, "option2 must omit/default prorated");
        let (r1, r2) = per_option(&bc.lowest_rate);
        assert_eq!(*r1, rate("0.0614"));
        assert_eq!(*r2, rate("0.0560"));
        let reduction = bc.tax_reduction.as_ref().expect("BC tax_reduction");
        let (s1, s2) = per_option(reduction);
        assert_eq!(s1.basic, money("805.00"));
        assert_eq!(s2.basic, money("690.00"));
        assert_eq!(s1.dependant, money("44952.00"));
        assert_eq!(s2.dependant, money("44952.00"));
        assert!(matches!(bc.brackets, OptionScoped::PerOption { .. }));
        assert!(matches!(bc.lowest_rate, OptionScoped::PerOption { .. }));
        assert!(matches!(reduction, OptionScoped::PerOption { .. }));
        assert!(!matches!(bc.brackets, OptionScoped::Both(_)));
    }

    /// Test 16 — NL July: brackets unchanged, BPA 15000 (option1) vs 13094 (option2).
    #[test]
    fn july_nl_brackets_unchanged_bpa_per_option() {
        let jan = load_ruleset_2026_01_01().unwrap();
        let july = load_ruleset_2026_07_01().unwrap();
        let jan_nl = j(&jan, "NL");
        let july_nl = j(&july, "NL");
        assert_eq!(
            jan_nl.brackets.get(CalculationOption::Option1),
            july_nl.brackets.get(CalculationOption::Option1)
        );
        assert!(matches!(july_nl.brackets, OptionScoped::Both(_)));
        let (bpa1, bpa2) = per_option(&july_nl.basic_personal_amount);
        match bpa1 {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("15000.00")),
            other => panic!("NL option1: {other:?}"),
        }
        match bpa2 {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("13094.00")),
            other => panic!("NL option2: {other:?}"),
        }
    }

    /// Test 17 — PE gains a sixth bracket in July. January has five.
    #[test]
    fn july_pe_gains_a_sixth_bracket() {
        let jan = load_ruleset_2026_01_01().unwrap();
        let july = load_ruleset_2026_07_01().unwrap();
        let jan_pe = j(&jan, "PE").brackets.get(CalculationOption::Option1);
        let (opt1, opt2) = per_option(&j(&july, "PE").brackets);
        assert_eq!(jan_pe.len(), 5, "January PE has five brackets");
        assert_eq!(opt1.len(), 6, "July PE option1 has six brackets");
        assert_eq!(opt2.len(), 6, "July PE option2 has six brackets");
        let sixth1 = &opt1[5];
        assert_eq!(sixth1.threshold, money("200000.00"));
        assert_eq!(sixth1.rate, rate("0.2100"));
        assert_eq!(sixth1.constant, money("10464.00"));
        assert!(sixth1.prorated);
        let sixth2 = &opt2[5];
        assert_eq!(sixth2.threshold, money("200000.00"));
        assert_eq!(sixth2.rate, rate("0.2000"));
        assert_eq!(sixth2.constant, money("8464.00"));
        assert!(!sixth2.prorated);
    }

    /// Test 18 — the ten unchanged jurisdictions are identical Jan vs July
    /// except RuleSet-level version / dates / provenance. If ON moved, overlay is wrong.
    #[test]
    fn ten_inherited_jurisdictions_are_byte_identical() {
        let jan = load_ruleset_2026_01_01().unwrap();
        let july = load_ruleset_2026_07_01().unwrap();
        assert_ne!(jan.rule_set_version, july.rule_set_version);
        assert_ne!(jan.effective_from, july.effective_from);
        assert_ne!(jan.source_document, july.source_document);
        const INHERITED: &[&str] = &["FED", "AB", "MB", "NB", "NS", "NT", "NU", "ON", "SK", "YT"];
        for code in INHERITED {
            let a = j(&jan, code);
            let b = j(&july, code);
            assert_eq!(a.brackets, b.brackets, "{code} brackets");
            assert_eq!(a.lowest_rate, b.lowest_rate, "{code} lowest_rate");
            assert_eq!(
                a.basic_personal_amount, b.basic_personal_amount,
                "{code} BPA"
            );
            assert_eq!(
                a.canada_employment_amount, b.canada_employment_amount,
                "{code} CEA"
            );
            assert_eq!(a.index_rate, b.index_rate, "{code} index_rate");
            assert_eq!(a.lcf, b.lcf, "{code} lcf");
            assert_eq!(a.lcp, b.lcp, "{code} lcp");
            assert_eq!(a.tax_reduction, b.tax_reduction, "{code} tax_reduction");
            assert_eq!(a.surtax, b.surtax, "{code} surtax");
            assert_eq!(a.health_premium, b.health_premium, "{code} health_premium");
            assert_eq!(
                a.supplemental_credit, b.supplemental_credit,
                "{code} supplemental_credit"
            );
            assert_eq!(
                a.employment_credit_rate, b.employment_credit_rate,
                "{code} employment_credit_rate"
            );
            assert_eq!(a.abatement, b.abatement, "{code} abatement");
            assert_eq!(a.surtax_flat, b.surtax_flat, "{code} surtax_flat");
            assert_eq!(a, b, "{code} whole jurisdiction");
        }
    }

    /// edition-identity.json lists exactly the jurisdictions test 18 proves identical.
    #[test]
    fn edition_identity_json_matches_test_18() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/edition-identity.json");
        let raw = std::fs::read_to_string(&path).expect("edition-identity.json");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let pair = &v["pairs"][0];
        assert_eq!(pair["a"], "2026-01-01");
        assert_eq!(pair["b"], "2026-07-01");
        let listed: Vec<String> = pair["identical_jurisdictions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        const INHERITED: &[&str] = &["FED", "AB", "MB", "NB", "NS", "NT", "NU", "ON", "SK", "YT"];
        assert_eq!(listed, INHERITED);
    }

    /// 48. Preview CPP rates, held YMPE, BC full-year 5.60% (not the 2026 Option 1 prorated 6.14%).
    #[test]
    fn preview_2027_cpp_and_bc_are_the_announced_full_year_tables() {
        let set = load_ruleset_2027_01_01().unwrap();
        assert_eq!(set.status, crate::rules::schema::RuleSetStatus::Proposed);
        assert_eq!(
            set.announcement_date
                .expect("announcement_date")
                .to_string(),
            "2026-04-28"
        );
        let cpp = &set.cpp;
        assert_eq!(cpp.ympe, money("74600.00"));
        assert_eq!(cpp.yampe, money("85000.00"));
        assert_eq!(cpp.total_rate, rate("0.0575"));
        assert_eq!(cpp.base_rate, rate("0.0475"));
        assert_eq!(cpp.total_max, money("4088.25"));
        assert_eq!(cpp.base_max, money("3377.25"));
        assert_eq!(cpp.first_additional_rate, rate("0.0100"));
        assert_eq!(cpp.first_additional_max, money("711.00"));
        assert_eq!(cpp.second_additional_max, money("416.00"));
        let bc = j(&set, "BC");
        assert_eq!(
            *bc.lowest_rate.get(CalculationOption::Option1),
            rate("0.0560")
        );
        assert!(!bc.brackets.get(CalculationOption::Option1)[0].prorated);
        match bc.basic_personal_amount.get(CalculationOption::Option1) {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("13216.00")),
            other => panic!("BC BPA: {other:?}"),
        }
        let reduction = bc
            .tax_reduction
            .as_ref()
            .unwrap()
            .get(CalculationOption::Option1);
        assert_eq!(reduction.basic, money("690.00"));
        match j(&set, "NL")
            .basic_personal_amount
            .get(CalculationOption::Option1)
        {
            BasicPersonalAmount::Fixed { amount } => assert_eq!(*amount, money("15000.00")),
            other => panic!("NL BPA: {other:?}"),
        }
        let pe = j(&set, "PE").brackets.get(CalculationOption::Option1);
        assert_eq!(pe[5].rate, rate("0.2000"));
        assert!(!pe[5].prorated);
    }

    #[test]
    fn preview_2027_inherited_files_match_july_on_disk() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/rules");
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
            "ei.json",
            "qpip.json",
        ] {
            let a = std::fs::read(root.join("2026-07-01").join(name)).expect(name);
            let b = std::fs::read(root.join("2027-01-01").join(name)).expect(name);
            assert_eq!(
                a, b,
                "{name} must stay a July copy until it has its own 2027 table"
            );
        }
    }
}
