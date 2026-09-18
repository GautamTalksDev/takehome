//! Rule-set schema (spec §9.1).
//!
//! Every numeric field is a lexical string at rest and parses to
//! [`Money`](crate::decimal::Money) / [`Rate`](crate::decimal::Rate) /
//! [`Ratio`](crate::decimal::Ratio) on load. Never a JSON number.

use crate::decimal::{Money, Rate};
use serde::de::{self, DeserializeOwned, Deserializer};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[cfg(test)]
mod tests {
    use super::{
        BasicPersonalAmount, CalculationOption, JurisdictionCode, LoadError, OptionScoped, RuleSet,
        RuleSetStatus,
    };
    use crate::decimal::Rate;
    use serde_json::Value;

    const MINIMAL_JSON: &str = r#"{
        "rule_set_version": "2026-01-01",
        "effective_from": "2026-01-01",
        "effective_to": null,
        "published_by": "CRA",
        "source_document": "T4127 Payroll Deductions Formulas",
        "source_url": "https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas.html",
        "retrieved_at": "2026-01-15T00:00:00Z",
        "source_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "jurisdictions": {
            "ON": {
                "brackets": [
                    {"threshold": "0", "rate": "0.0505", "constant": "0.00"}
                ],
                "lowest_rate": "0.0505",
                "basic_personal_amount": {"type": "fixed", "amount": "12747.00"}
            }
        },
        "cpp": {
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
        },
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
    }"#;

    fn load(json: &str) -> Result<RuleSet, LoadError> {
        RuleSet::from_json(json)
    }

    fn assert_no_json_numbers(value: &Value, path: &str) {
        match value {
            Value::Number(n) => panic!("numeric JSON at {path}: {n}; numerics must be strings"),
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    assert_no_json_numbers(item, &format!("{path}[{i}]"));
                }
            }
            Value::Object(map) => {
                for (k, v) in map {
                    assert_no_json_numbers(v, &format!("{path}.{k}"));
                }
            }
            Value::Null | Value::Bool(_) | Value::String(_) => {}
        }
    }

    // --- SCHEMA / SERDE ----------------------------------------------------

    /// 1. A minimal RuleSet JSON with every numeric as a string deserialises.
    #[test]
    fn minimal_ruleset_json_deserialises_with_string_numerics() {
        let rs = load(MINIMAL_JSON).expect("minimal RuleSet must load");
        assert_eq!(rs.rule_set_version, "2026-01-01");
        assert_eq!(rs.effective_from.to_string(), "2026-01-01");
        assert_eq!(rs.effective_to, None);
        assert_eq!(rs.status, RuleSetStatus::Enacted);
        assert_eq!(rs.announcement_source, None);
        assert_eq!(rs.announcement_date, None);
        let on = rs
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .expect("ON");
        match &on.basic_personal_amount {
            OptionScoped::Both(BasicPersonalAmount::Fixed { amount }) => {
                assert_eq!(amount.to_string(), "12747.00");
            }
            other => panic!("expected Both(Fixed), got {other:?}"),
        }
    }

    /// Spec open question 5: omitted status is enacted so existing JSON still loads.
    #[test]
    fn proposed_status_requires_announcement_source_and_date() {
        let json = MINIMAL_JSON.replace(
            "\"source_sha256\"",
            "\"status\": \"proposed\", \"source_sha256\"",
        );
        let err = load(&json).expect_err("proposed without announcement must fail");
        assert!(err.to_string().contains("announcement"), "got {}", err);
    }

    #[test]
    fn proposed_status_loads_with_announcement() {
        let json = MINIMAL_JSON.replace(
            "\"source_sha256\"",
            concat!(
                "\"status\": \"proposed\", ",
                "\"announcement_source\": \"Department of Finance Canada, Spring Economic Update 2026\", ",
                "\"announcement_date\": \"2026-04-28\", ",
                "\"source_sha256\""
            ),
        );
        let rs = load(&json).expect("proposed with announcement must load");
        assert_eq!(rs.status, RuleSetStatus::Proposed);
        assert_eq!(
            rs.announcement_source.as_deref(),
            Some("Department of Finance Canada, Spring Economic Update 2026")
        );
        assert_eq!(
            rs.announcement_date.expect("date").to_string(),
            "2026-04-28"
        );
    }

    /// 2. A JSON *number* for a numeric field is a hard error naming that field.
    #[test]
    fn json_number_for_rate_fails_and_names_the_field() {
        let json = MINIMAL_JSON.replace("\"rate\": \"0.0505\"", "\"rate\": 0.14");
        let err = load(&json).expect_err("JSON numbers must not load");
        let msg = err.to_string();
        assert!(
            msg.contains("rate"),
            "error must name the field `rate`, got {msg}"
        );
    }

    /// 3. Round-trip: load, serialise, load. Serialisation emits strings, never numbers.
    #[test]
    fn ruleset_round_trip_emits_strings_never_numbers() {
        let first = load(MINIMAL_JSON).expect("load");
        let encoded = serde_json::to_value(&first).expect("serialise");
        assert_no_json_numbers(&encoded, "$");
        let encoded_str = serde_json::to_string(&encoded).expect("stringify");
        let second = load(&encoded_str).expect("reload");
        assert_eq!(first, second);
    }

    /// 4. Unknown field anywhere in the tree is a hard error.
    #[test]
    fn unknown_field_anywhere_is_a_hard_error() {
        let top = MINIMAL_JSON.replace(
            "\"rule_set_version\"",
            "\"typo_version\": \"nope\", \"rule_set_version\"",
        );
        let err = load(&top).expect_err("unknown top-level field");
        let msg = err.to_string();
        assert!(
            msg.contains("typo_version"),
            "error must name the unknown field, got {msg}"
        );

        let nested = MINIMAL_JSON.replace(
            "\"constant\": \"0.00\"",
            "\"constant\": \"0.00\", \"typo_constant\": \"nope\"",
        );
        let err = load(&nested).expect_err("unknown nested field");
        let msg = err.to_string();
        assert!(
            msg.contains("typo_constant"),
            "error must name the unknown nested field, got {msg}"
        );
    }

    /// 5. Missing required field errors name the field path.
    #[test]
    fn missing_required_field_errors_name_the_field_path() {
        let missing_top =
            MINIMAL_JSON.replace("\"rule_set_version\": \"2026-01-01\",\n        ", "");
        let err = load(&missing_top).expect_err("missing rule_set_version");
        let msg = err.to_string();
        assert!(
            msg.contains("rule_set_version"),
            "error must name the field path, got {msg}"
        );

        let missing_nested = MINIMAL_JSON.replace(", \"constant\": \"0.00\"", "");
        let err = load(&missing_nested).expect_err("missing constant");
        let msg = err.to_string();
        assert!(
            msg.contains("constant"),
            "error must name the nested field path, got {msg}"
        );
    }

    // --- OPTION SCOPING (spec §21.3) ---------------------------------------

    /// 6. Both(T) returns the same value for Option1 and Option2.
    #[test]
    fn option_scoped_both_returns_same_rate_for_each_option() {
        let scoped = OptionScoped::Both(Rate::parse("0.0506").unwrap());
        assert_eq!(
            scoped.get(CalculationOption::Option1),
            scoped.get(CalculationOption::Option2)
        );
        assert_eq!(
            *scoped.get(CalculationOption::Option1),
            Rate::parse("0.0506").unwrap()
        );
    }

    /// Test 7: PerOption returns different rates. This is BC July 2026; flattening
    /// this makes Option 2 silently wrong for the whole second half of any year
    /// with a mid-year change.
    #[test]
    fn option_scoped_per_option_returns_distinct_rates() {
        let scoped = OptionScoped::PerOption {
            option1: Rate::parse("0.0614").unwrap(),
            option2: Rate::parse("0.0560").unwrap(),
        };
        assert_eq!(
            *scoped.get(CalculationOption::Option1),
            Rate::parse("0.0614").unwrap()
        );
        assert_eq!(
            *scoped.get(CalculationOption::Option2),
            Rate::parse("0.0560").unwrap()
        );
        assert_ne!(
            scoped.get(CalculationOption::Option1),
            scoped.get(CalculationOption::Option2)
        );
    }

    /// 8. `"same_as_option1"` in the option2 slot resolves to option1's value.
    #[test]
    fn same_as_option1_resolves_to_option1_value() {
        let json = MINIMAL_JSON.replace(
            "\"lowest_rate\": \"0.0505\"",
            "\"lowest_rate\": {\"option1\": \"0.0614\", \"option2\": \"same_as_option1\"}",
        );
        let rs = load(&json).expect("same_as_option1 must resolve at load");
        let on = rs
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .expect("ON");
        let rate = &on.lowest_rate;
        assert_eq!(
            *rate.get(CalculationOption::Option1),
            Rate::parse("0.0614").unwrap()
        );
        assert_eq!(
            *rate.get(CalculationOption::Option2),
            Rate::parse("0.0614").unwrap()
        );
    }

    /// 9. `prorated: true` round-trips; default is false when the key is absent.
    #[test]
    fn bracket_prorated_flag_round_trips_and_defaults_false() {
        let absent = load(MINIMAL_JSON).expect("load");
        let on = absent
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .expect("ON");
        let brackets = on.brackets.get(CalculationOption::Option1);
        assert!(
            !brackets[0].prorated,
            "absent prorated key defaults to false"
        );

        let with_flag = MINIMAL_JSON.replace(
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\"}",
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\", \"prorated\": true}",
        );
        let loaded = load(&with_flag).expect("load prorated");
        let on = loaded
            .jurisdictions
            .get(&JurisdictionCode("ON".to_string()))
            .expect("ON");
        assert!(on.brackets.get(CalculationOption::Option1)[0].prorated);

        let encoded = serde_json::to_string(&loaded).expect("serialise");
        let reloaded = load(&encoded).expect("reload");
        assert_eq!(loaded, reloaded);
        assert!(
            reloaded
                .jurisdictions
                .get(&JurisdictionCode("ON".to_string()))
                .unwrap()
                .brackets
                .get(CalculationOption::Option1)[0]
                .prorated
        );
    }

    // --- BRACKETS ----------------------------------------------------------

    /// 10. Unsorted brackets are a load error, not silently sorted.
    #[test]
    fn unsorted_brackets_are_a_load_error() {
        let json = MINIMAL_JSON.replace(
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\"}",
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\"}, \
             {\"threshold\": \"100000.00\", \"rate\": \"0.1306\", \"constant\": \"0.00\"}, \
             {\"threshold\": \"50000.00\", \"rate\": \"0.0915\", \"constant\": \"0.00\"}",
        );
        let err = load(&json).expect_err("unsorted brackets must not load");
        let msg = err.to_string();
        assert!(
            msg.contains("threshold") || msg.contains("sort") || msg.contains("ascending"),
            "unsorted-bracket error must name the invariant, got {msg}"
        );
    }

    /// 11. First bracket threshold must be `"0"`; otherwise load error.
    #[test]
    fn first_bracket_threshold_must_be_zero() {
        let json = MINIMAL_JSON.replace("\"threshold\": \"0\"", "\"threshold\": \"1.00\"");
        let err = load(&json).expect_err("non-zero first threshold must not load");
        let msg = err.to_string();
        assert!(
            msg.contains("threshold") || msg.contains('0'),
            "error must name the first-threshold invariant, got {msg}"
        );
    }

    /// 12. Duplicate thresholds are a load error.
    #[test]
    fn duplicate_bracket_thresholds_are_a_load_error() {
        let json = MINIMAL_JSON.replace(
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\"}",
            "{\"threshold\": \"0\", \"rate\": \"0.0505\", \"constant\": \"0.00\"}, \
             {\"threshold\": \"0\", \"rate\": \"0.0915\", \"constant\": \"0.00\"}",
        );
        let err = load(&json).expect_err("duplicate thresholds must not load");
        let msg = err.to_string();
        assert!(
            msg.contains("duplicate") || msg.contains("threshold"),
            "error must name the duplicate-threshold invariant, got {msg}"
        );
    }

    #[test]
    fn calendar_date_parses_iso_and_rejects_invalid_civil_dates() {
        assert!("2026-01-01".parse::<super::CalendarDate>().is_ok());
        assert!("2024-02-29".parse::<super::CalendarDate>().is_ok());
        assert!("2026-02-30".parse::<super::CalendarDate>().is_err());
        assert!("2026-13-01".parse::<super::CalendarDate>().is_err());
        assert!("2026-02-29".parse::<super::CalendarDate>().is_err());
        assert!("2026-1-1".parse::<super::CalendarDate>().is_err());
    }
}

/// Civil date `(year, month, day)`. Data, not a clock (spec §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// Failed `YYYY-MM-DD` parse or impossible civil date.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DateParseError {
    /// Wrong shape (not `YYYY-MM-DD` with two-digit month and day).
    #[error("invalid calendar date {input}")]
    InvalidFormat { input: String },
    /// Well-formed digits that are not a real civil date (e.g. 2026-02-30).
    #[error("invalid calendar date {year:04}-{month:02}-{day:02}")]
    InvalidYmd { year: u16, month: u8, day: u8 },
}

impl CalendarDate {
    /// Construct a validated civil date. No clock is read.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DateParseError> {
        if (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month) {
            Ok(Self { year, month, day })
        } else {
            Err(DateParseError::InvalidYmd { year, month, day })
        }
    }
}

fn is_leap_year(year: u16) -> bool {
    let y = u32::from(year);
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for CalendarDate {
    type Err = DateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || DateParseError::InvalidFormat {
            input: s.to_string(),
        };
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(invalid());
        }
        if !bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => true,
            _ => b.is_ascii_digit(),
        }) {
            return Err(invalid());
        }
        let year: u16 = s[0..4].parse().map_err(|_| invalid())?;
        let month: u8 = s[5..7].parse().map_err(|_| invalid())?;
        let day: u8 = s[8..10].parse().map_err(|_| invalid())?;
        Self::new(year, month, day)
    }
}

impl Serialize for CalendarDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for CalendarDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

/// Jurisdiction map key (`FED`, `ON`, `BC`, …).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JurisdictionCode(pub String);

/// T4127 Option 1 vs Option 2 (spec §21.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculationOption {
    Option1,
    Option2,
}

/// Value that is either shared across options or distinct per option.
///
/// Serde accepts a bare value ([`Self::Both`]) and
/// `{"option1": …, "option2": …}`. The literal `"same_as_option1"` in
/// `option2` resolves to option1's value at load (spec §21.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionScoped<T> {
    Both(T),
    PerOption { option1: T, option2: T },
}

impl<T> OptionScoped<T> {
    /// Value in force for this calculation option (T4127 Option 1 / Option 2; spec §21.3).
    pub fn get(&self, opt: CalculationOption) -> &T {
        match (self, opt) {
            (Self::Both(value), _) => value,
            (Self::PerOption { option1, .. }, CalculationOption::Option1) => option1,
            (Self::PerOption { option2, .. }, CalculationOption::Option2) => option2,
        }
    }
}

impl<T: Serialize> Serialize for OptionScoped<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Both(value) => value.serialize(serializer),
            Self::PerOption { option1, option2 } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("option1", option1)?;
                map.serialize_entry("option2", option2)?;
                map.end()
            }
        }
    }
}

impl<'de, T> Deserialize<'de> for OptionScoped<T>
where
    T: DeserializeOwned + Clone,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let serde_json::Value::Object(map) = &value {
            if map.contains_key("option1") || map.contains_key("option2") {
                return deserialize_per_option(value).map_err(de::Error::custom);
            }
        }
        match serde_path_to_error::deserialize(value) {
            Ok(inner) => Ok(Self::Both(inner)),
            Err(err) => Err(de::Error::custom(err.to_string())),
        }
    }
}

fn deserialize_per_option<T>(value: serde_json::Value) -> Result<OptionScoped<T>, String>
where
    T: DeserializeOwned + Clone,
{
    let serde_json::Value::Object(map) = value else {
        return Err("option-scoped object required".to_string());
    };
    for key in map.keys() {
        if key != "option1" && key != "option2" {
            return Err(format!(
                "unknown field `{key}`, expected `option1` or `option2`"
            ));
        }
    }
    let option1_value = map
        .get("option1")
        .cloned()
        .ok_or_else(|| "missing field `option1`".to_string())?;
    let option2_value = map
        .get("option2")
        .cloned()
        .ok_or_else(|| "missing field `option2`".to_string())?;
    let option1: T =
        serde_path_to_error::deserialize(option1_value).map_err(|err| err.to_string())?;
    let option2 = if option2_value.as_str() == Some("same_as_option1") {
        option1.clone()
    } else {
        serde_path_to_error::deserialize(option2_value).map_err(|err| err.to_string())?
    };
    Ok(OptionScoped::PerOption { option1, option2 })
}

/// Whether a rule set is a published T4127 edition or a labelled preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSetStatus {
    #[default]
    Enacted,
    Proposed,
}

/// One effective-dated T4127 rule set (spec §9.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub rule_set_version: String,
    pub effective_from: CalendarDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_to: Option<CalendarDate>,
    pub published_by: String,
    pub source_document: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub source_sha256: String,
    /// Whether this edition is CRA-enacted T4127 or a labelled preview (spec open question 5).
    #[serde(default)]
    pub status: RuleSetStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement_date: Option<CalendarDate>,
    pub jurisdictions: BTreeMap<JurisdictionCode, Jurisdiction>,
    pub cpp: CppParams,
    pub ei: EiParams,
    pub qpip: QpipParams,
}

impl RuleSet {
    /// Load a rule set from JSON. Numerics must be lexical strings (spec §9.1).
    pub fn from_json(json: &str) -> Result<Self, LoadError> {
        let mut deserializer = serde_json::Deserializer::from_str(json);
        let loaded: RuleSet =
            serde_path_to_error::deserialize(&mut deserializer).map_err(|err| LoadError {
                message: err.to_string(),
            })?;
        loaded.validate()?;
        Ok(loaded)
    }

    /// Re-check bracket invariants after assembling a set from fragments.
    pub(crate) fn validate_assembled(&self) -> Result<(), LoadError> {
        self.validate()
    }

    fn validate(&self) -> Result<(), LoadError> {
        for (code, jurisdiction) in &self.jurisdictions {
            match &jurisdiction.brackets {
                OptionScoped::Both(brackets) => validate_brackets(code, "both", brackets)?,
                OptionScoped::PerOption { option1, option2 } => {
                    validate_brackets(code, "option1", option1)?;
                    validate_brackets(code, "option2", option2)?;
                }
            }
        }
        if self.status == RuleSetStatus::Proposed {
            let source_ok = self
                .announcement_source
                .as_deref()
                .is_some_and(|source| !source.is_empty());
            if !source_ok || self.announcement_date.is_none() {
                return Err(LoadError {
                    message: "status proposed requires announcement_source and announcement_date"
                        .to_string(),
                });
            }
        }
        Ok(())
    }
}

fn validate_brackets(
    jurisdiction: &JurisdictionCode,
    scope: &str,
    brackets: &[Bracket],
) -> Result<(), LoadError> {
    if brackets.is_empty() {
        return Err(LoadError {
            message: format!(
                "jurisdiction {} ({scope}): brackets must start at threshold 0",
                jurisdiction.0
            ),
        });
    }
    if !brackets[0].threshold.is_zero() {
        return Err(LoadError {
            message: format!(
                "jurisdiction {} ({scope}): first bracket threshold must be 0",
                jurisdiction.0
            ),
        });
    }
    for pair in brackets.windows(2) {
        match pair[0].threshold.cmp(&pair[1].threshold) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                return Err(LoadError {
                    message: format!(
                        "jurisdiction {} ({scope}): duplicate bracket threshold {}",
                        jurisdiction.0, pair[1].threshold
                    ),
                });
            }
            std::cmp::Ordering::Greater => {
                return Err(LoadError {
                    message: format!(
                        "jurisdiction {} ({scope}): brackets are not sorted ascending by threshold",
                        jurisdiction.0
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Load-time schema failure (unknown fields, JSON numbers, bracket invariants).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct LoadError {
    pub message: String,
}

/// One jurisdiction's tax table and credits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Jurisdiction {
    pub brackets: OptionScoped<Vec<Bracket>>,
    pub lowest_rate: OptionScoped<Rate>,
    pub basic_personal_amount: OptionScoped<BasicPersonalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canada_employment_amount: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_rate: Option<Rate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lcf: Option<LabourCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lcp: Option<LabourCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_reduction: Option<OptionScoped<TaxReduction>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surtax: Option<Vec<SurtaxTier>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_premium: Option<Vec<HealthPremiumTier>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplemental_credit: Option<SupplementalCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employment_credit_rate: Option<Rate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abatement: Option<Rate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surtax_flat: Option<Rate>,
}

/// One tax bracket. `prorated` defaults to false when absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    pub threshold: Money,
    pub rate: Rate,
    pub constant: Money,
    #[serde(default)]
    pub prorated: bool,
}

/// Basic personal amount (fixed, income-phased, or a provincial alias).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BasicPersonalAmount {
    Fixed {
        amount: Money,
    },
    Dynamic {
        formula: BpaFormula,
        max: Money,
        min: Money,
        phaseout_start: Money,
        phaseout_end: Money,
    },
    SameAsFederal,
    NotApplicable,
}

/// Federal CPP / QPP contribution parameters (T4127 Tables 8.3–8.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CppParams {
    pub ympe: Money,
    pub yampe: Money,
    pub basic_exemption: Money,
    pub total_rate: Rate,
    pub total_max: Money,
    pub base_rate: Rate,
    pub base_max: Money,
    pub first_additional_rate: Rate,
    pub first_additional_max: Money,
    pub second_additional_rate: Rate,
    pub second_additional_max: Money,
}

/// Employment insurance parameters (T4127 Table 8.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EiParams {
    pub max_insurable: Money,
    pub employee_rate: Rate,
    pub employee_max: Money,
    pub employer_rate: Rate,
    pub employer_max: Money,
}

/// Québec Parental Insurance Plan parameters (T4127 Table 8.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QpipParams {
    pub max_insurable: Money,
    pub employee_rate: Rate,
    pub employee_max: Money,
    pub employer_rate: Rate,
    pub employer_max: Money,
}

/// Labour-sponsored funds tax credit (LCF / LCP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabourCredit {
    pub rate: Rate,
    pub max: Money,
}

/// Provincial tax reduction (factor S / S2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxReduction {
    /// Basic reduction amount S2 (T4127 Table 8.2).
    pub basic: Money,
    /// Per-dependant amount for factor Y (T4127 Chapter 4, Ontario).
    pub dependant: Money,
}

/// Ontario V1 surtax tier (threshold on T4, rate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurtaxTier {
    pub threshold: Money,
    pub rate: Rate,
}

/// Ontario V2 health premium tier (threshold on A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthPremiumTier {
    pub threshold: Money,
    pub rate: Rate,
    pub base: Money,
    pub cap: Money,
}

/// Alberta K5P supplemental credit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplementalCredit {}

/// BPA dynamic formula identity (BPAF / BPAMB / BPAYT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BpaFormula {}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(LexicalVisitor {
            parse: Money::parse,
        })
    }
}

impl Serialize for Rate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Rate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(LexicalVisitor { parse: Rate::parse })
    }
}

struct LexicalVisitor<T, E> {
    parse: fn(&str) -> Result<T, E>,
}

impl<'de, T, E> de::Visitor<'de> for LexicalVisitor<T, E>
where
    E: fmt::Display,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a lexical decimal string")
    }

    fn visit_str<Er: de::Error>(self, v: &str) -> Result<T, Er> {
        (self.parse)(v).map_err(Er::custom)
    }

    fn visit_string<Er: de::Error>(self, v: String) -> Result<T, Er> {
        self.visit_str(&v)
    }
}
