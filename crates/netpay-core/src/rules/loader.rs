//! Compile-time embedding of hand-authored rule JSON (spec §9.1).
//!
//! `netpay-core` stays IO-free: every byte arrives through `include_str!`.
//! Provenance lives in `manifest.json` (source URL, retrieved_at, sha256).

use crate::rules::registry::{Registry, RuleError};
use crate::rules::schema::{
    CppParams, EiParams, Jurisdiction, JurisdictionCode, LoadError, QpipParams, RuleSet,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use thiserror::Error;

const MANIFEST_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/manifest.json"
));
const FEDERAL_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/federal.json"
));
const ON_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/on.json"
));
const CPP_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/cpp.json"
));
const EI_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/ei.json"
));
const QPIP_2026_01_01: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/rules/2026-01-01/qpip.json"
));

/// Lazily assembled registry of every embedded rule set.
pub static EMBEDDED_REGISTRY: LazyLock<Registry> =
    LazyLock::new(|| load_embedded_registry().expect("embedded rule sets must load"));

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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestSource {
    source_document: String,
    source_url: String,
    retrieved_at: String,
    #[serde(default)]
    scalars_verified_at: Option<String>,
    source_sha256: String,
    files: Vec<String>,
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
    let mut deserializer = serde_json::Deserializer::from_str(MANIFEST_2026_01_01);
    let manifest: Manifest = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|err| EmbedError::Manifest(err.to_string()))?;
    let source = manifest
        .sources
        .first()
        .ok_or_else(|| EmbedError::Manifest("manifest has no sources".to_string()))?;
    for required in [
        "federal.json",
        "on.json",
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
    jurisdictions.insert(
        JurisdictionCode("FED".to_string()),
        load_jurisdiction(FEDERAL_2026_01_01, "FED")?,
    );
    jurisdictions.insert(
        JurisdictionCode("ON".to_string()),
        load_jurisdiction(ON_2026_01_01, "ON")?,
    );

    let cpp: CppParams = load_component(CPP_2026_01_01, EmbedError::Cpp)?;
    let ei: EiParams = load_component(EI_2026_01_01, EmbedError::Ei)?;
    let qpip: QpipParams = load_component(QPIP_2026_01_01, EmbedError::Qpip)?;

    let set = RuleSet {
        rule_set_version: manifest.rule_set_version,
        effective_from: manifest.effective_from,
        effective_to: manifest.effective_to,
        published_by: manifest.published_by,
        source_document: source.source_document.clone(),
        source_url: source.source_url.clone(),
        retrieved_at: source.retrieved_at.clone(),
        source_sha256: source.source_sha256.clone(),
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
    Registry::load(vec![load_ruleset_2026_01_01()?]).map_err(EmbedError::from)
}

#[cfg(test)]
mod tests {
    use super::{load_ruleset_2026_01_01, EMBEDDED_REGISTRY};
    use crate::decimal::{Money, Rate};
    use crate::rounding::{round_claim_to_dollar, round_contribution_to_cent};
    use crate::rules::schema::{
        BasicPersonalAmount, CalculationOption, CppParams, EiParams, JurisdictionCode,
        OptionScoped, QpipParams,
    };

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn rate(s: &str) -> Rate {
        Rate::parse(s).unwrap()
    }

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
    // Covered: every Table 8.1 threshold, rate, and constant in federal and
    // Ontario. A mistype in any of those moves the residual and fails.
    //
    // Not covered: scalars that do not participate in the bracket identity
    // (BPAF min/max/phaseout, CEA, LCF, Ontario S2/Y/V1/V2, CPP, EI, QPIP).
    // Those are ticked against T4127 in
    // `non_bracket_scalars_match_t4127_2026_01_01` and algebraically in 28b.
    #[test]
    fn bracket_k_constants_match_cumulative_rate_delta_identity() {
        let set = load_ruleset_2026_01_01().unwrap();
        for (code, jurisdiction) in &set.jurisdictions {
            match &jurisdiction.brackets {
                OptionScoped::Both(brackets) => {
                    assert_k_identity(&code.0, "both", brackets);
                }
                OptionScoped::PerOption { option1, option2 } => {
                    assert_k_identity(&code.0, "option1", option1);
                    assert_k_identity(&code.0, "option2", option2);
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
        let set = load_ruleset_2026_01_01().unwrap();
        assert_cpp_identities("CPP Table 8.3–8.6", &set.cpp);
        assert_ei_identities("EI Table 8.7 Canada except QC", &set.ei);
        assert_qpip_identities(&set.qpip);

        // QPP / Quebec EI are not yet a rule-set jurisdiction (M1 is Ontario),
        // but Table 8.3–8.7 QC rows must still satisfy the same identities.
        let mut qpp = set.cpp.clone();
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

    #[test]
    fn scalars_verified_at_is_recorded_on_the_manifest() {
        let manifest: super::Manifest =
            serde_json::from_str(super::MANIFEST_2026_01_01).expect("manifest");
        assert_eq!(
            manifest.sources[0].scalars_verified_at.as_deref(),
            Some("2026-09-12T02:50:00Z")
        );
    }
}
