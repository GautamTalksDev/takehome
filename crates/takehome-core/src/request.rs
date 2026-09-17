//! Deduction request (spec §9.3). Public wire contract — shapes do not change lightly.
//!
//! `as_of` has no default inside core (no clock). Callers must supply it; the API
//! service layer may default it to today and must say so.

use crate::decimal::Money;
use crate::rules::schema::{CalculationOption, CalendarDate};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[cfg(test)]
mod tests {
    use super::{
        CalculationOption, ClaimCode, K2Method, PayPeriod, Province, Request, RequestError,
        RoundingCompat,
    };

    fn minimal_json() -> String {
        r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00"
        }"#
        .to_string()
    }

    /// 32. Deserialising a request with an unknown field errors and names the field.
    #[test]
    fn unknown_request_field_errors_and_names_the_field() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "typo_gross": "1.00"
        }"#;
        let err = Request::from_json(json).expect_err("unknown field");
        let msg = err.to_string();
        assert!(
            msg.contains("typo_gross"),
            "error must name the unknown field, got {msg}"
        );
    }

    /// 33. Supplying federal_claim_code AND federal_tc errors.
    #[test]
    fn federal_claim_code_and_federal_tc_together_is_an_error() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "federal_claim_code": 1,
            "federal_tc": "16452.00"
        }"#;
        let err = Request::from_json(json).expect_err("ambiguous claim");
        assert!(
            matches!(err, RequestError::AmbiguousFederalClaim),
            "got {err:?}"
        );
    }

    /// 34. pay_period 25 errors and the message lists the legal values.
    #[test]
    fn pay_period_25_errors_listing_legal_values() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 25,
            "gross_pay": "2500.00"
        }"#;
        let err = Request::from_json(json).expect_err("illegal pay period");
        let msg = err.to_string();
        assert!(msg.contains("25"), "got {msg}");
        for legal in PayPeriod::LEGAL {
            assert!(
                msg.contains(&legal.to_string()),
                "error must list legal value {legal}, got {msg}"
            );
        }
    }

    #[test]
    fn defaults_option1_cpp_months_12_k2_pdoc_observed_earnings_follow_gross() {
        let req = Request::from_json(&minimal_json()).expect("minimal");
        assert_eq!(req.calculation_option, CalculationOption::Option1);
        assert_eq!(req.cpp_months, 12);
        assert_eq!(req.k2_method, K2Method::PdocObserved);
        assert_eq!(req.rounding_compat, RoundingCompat::T4127);
        assert_eq!(req.pensionable_earnings(), &req.gross_pay);
        assert_eq!(req.insurable_earnings(), &req.gross_pay);
        assert!(req.as_of.to_string() == "2026-03-15");
        assert_eq!(req.dependants_under_19, None);
        assert_eq!(req.dependants_disabled, None);
    }

    #[test]
    fn k2_method_wire_names_and_annualized_alias() {
        fn parse(method: &str) -> K2Method {
            let json = format!(
                r#"{{
                    "as_of": "2026-03-15",
                    "province": "ON",
                    "pay_period": 26,
                    "gross_pay": "2500.00",
                    "k2_method": "{method}"
                }}"#
            );
            Request::from_json(&json).expect(method).k2_method
        }
        assert_eq!(parse("pdoc_observed"), K2Method::PdocObserved);
        assert_eq!(parse("annualized"), K2Method::PdocObserved);
        assert_eq!(parse("t4127_literal"), K2Method::T4127Literal);
        assert_eq!(parse("year_to_date"), K2Method::YearToDate);
    }

    #[test]
    fn unknown_province_names_field_and_lists_valid_values() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ZZ",
            "pay_period": 26,
            "gross_pay": "2500.00"
        }"#;
        let err = Request::from_json(json).expect_err("bad province");
        let msg = err.to_string();
        assert!(msg.contains("province"), "{msg}");
        assert!(msg.contains("ON"), "{msg}");
        assert!(msg.contains("OutsideCanada"), "{msg}");
    }

    #[test]
    fn claim_code_e_and_code_ten_parse() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "federal_claim_code": "E",
            "provincial_claim_code": 10
        }"#;
        let req = Request::from_json(json).unwrap();
        assert_eq!(req.federal_claim_code, Some(ClaimCode::E));
        assert_eq!(req.provincial_claim_code, Some(ClaimCode::Code(10)));
    }

    #[test]
    fn provincial_claim_code_and_tcp_together_is_an_error() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "provincial_claim_code": 1,
            "provincial_tcp": "12989.00"
        }"#;
        let err = Request::from_json(json).expect_err("ambiguous provincial claim");
        assert!(matches!(err, RequestError::AmbiguousProvincialClaim));
    }

    #[test]
    fn fifty_three_week_and_twenty_seven_biweekly_are_legal() {
        // Spec §21.10: 53-week and 27-biweekly years are real.
        assert!(PayPeriod::new(53).is_ok());
        assert!(PayPeriod::new(27).is_ok());
        assert_eq!(Province::OutsideCanada.as_str(), "OutsideCanada");
    }

    /// Spec §9.3: dependants_under_19 / dependants_disabled, not ontario_* or under_18.
    #[test]
    fn dependants_fields_match_t4127_y_cutoff() {
        let json = r#"{
            "as_of": "2026-03-15",
            "province": "ON",
            "pay_period": 26,
            "gross_pay": "2500.00",
            "dependants_under_19": 2,
            "dependants_disabled": 1
        }"#;
        let req = Request::from_json(json).unwrap();
        assert_eq!(req.dependants_under_19, Some(2));
        assert_eq!(req.dependants_disabled, Some(1));
    }

    #[test]
    fn ontario_prefixed_dependant_fields_are_unknown() {
        for field in [
            "ontario_dependants_under_18",
            "ontario_disabled_dependants",
            "dependants_under_18",
        ] {
            let json = format!(
                r#"{{
                    "as_of": "2026-03-15",
                    "province": "ON",
                    "pay_period": 26,
                    "gross_pay": "2500.00",
                    "{field}": 1
                }}"#
            );
            let err = Request::from_json(&json).expect_err("old field name");
            let msg = err.to_string();
            assert!(
                msg.contains(field),
                "error must name the unknown field {field}, got {msg}"
            );
        }
    }
}

/// Actionable request validation / parse failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RequestError {
    #[error("{message}")]
    Serde { message: String },
    #[error(
        "invalid province {got:?}: valid values are AB, BC, MB, NB, NL, NS, NT, NU, ON, PE, QC, SK, YT, OutsideCanada"
    )]
    UnknownProvince { got: String },
    #[error(
        "invalid pay_period {got}: legal values are 1, 2, 4, 10, 12, 13, 22, 24, 26, 27, 52, 53, 240, 2000 (spec §21.10; 53-week and 27-biweekly years are real)"
    )]
    InvalidPayPeriod { got: u16 },
    #[error("invalid claim_code {got:?}: use 0..=10 or E")]
    InvalidClaimCode { got: String },
    #[error(
        "ambiguous federal claim: supply federal_claim_code or federal_tc, not both (field conflict)"
    )]
    AmbiguousFederalClaim,
    #[error(
        "ambiguous provincial claim: supply provincial_claim_code or provincial_tcp, not both (field conflict)"
    )]
    AmbiguousProvincialClaim,
    /// `rounding_compat: pdoc` is accepted on the wire (ADR-003) but has no
    /// implementation until finding 002 names PDOC’s midpoint condition.
    #[error(
        "rounding_compat pdoc is not implemented until finding 002 names the PDOC midpoint rule (ADR-003)"
    )]
    RoundingCompatPdocNotImplemented,
}

impl Serialize for Province {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Province / territory of employment, plus OutsideCanada (T4127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Province {
    Ab,
    Bc,
    Mb,
    Nb,
    Nl,
    Ns,
    Nt,
    Nu,
    On,
    Pe,
    Qc,
    Sk,
    Yt,
    OutsideCanada,
}

impl Province {
    /// Wire / documentation spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ab => "AB",
            Self::Bc => "BC",
            Self::Mb => "MB",
            Self::Nb => "NB",
            Self::Nl => "NL",
            Self::Ns => "NS",
            Self::Nt => "NT",
            Self::Nu => "NU",
            Self::On => "ON",
            Self::Pe => "PE",
            Self::Qc => "QC",
            Self::Sk => "SK",
            Self::Yt => "YT",
            Self::OutsideCanada => "OutsideCanada",
        }
    }

    const ALL: &'static [Province] = &[
        Self::Ab,
        Self::Bc,
        Self::Mb,
        Self::Nb,
        Self::Nl,
        Self::Ns,
        Self::Nt,
        Self::Nu,
        Self::On,
        Self::Pe,
        Self::Qc,
        Self::Sk,
        Self::Yt,
        Self::OutsideCanada,
    ];

    /// Every province, territory, Quebec, and Outside Canada.
    pub fn all() -> &'static [Province] {
        Self::ALL
    }

    /// English display name for product surfaces.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ab => "Alberta",
            Self::Bc => "British Columbia",
            Self::Mb => "Manitoba",
            Self::Nb => "New Brunswick",
            Self::Nl => "Newfoundland and Labrador",
            Self::Ns => "Nova Scotia",
            Self::Nt => "Northwest Territories",
            Self::Nu => "Nunavut",
            Self::On => "Ontario",
            Self::Pe => "Prince Edward Island",
            Self::Qc => "Quebec",
            Self::Sk => "Saskatchewan",
            Self::Yt => "Yukon",
            Self::OutsideCanada => "Outside Canada",
        }
    }
}

impl fmt::Display for Province {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Province {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        match raw.as_str() {
            "AB" => Ok(Self::Ab),
            "BC" => Ok(Self::Bc),
            "MB" => Ok(Self::Mb),
            "NB" => Ok(Self::Nb),
            "NL" => Ok(Self::Nl),
            "NS" => Ok(Self::Ns),
            "NT" => Ok(Self::Nt),
            "NU" => Ok(Self::Nu),
            "ON" => Ok(Self::On),
            "PE" => Ok(Self::Pe),
            "QC" => Ok(Self::Qc),
            "SK" => Ok(Self::Sk),
            "YT" => Ok(Self::Yt),
            "OutsideCanada" => Ok(Self::OutsideCanada),
            other => Err(de::Error::custom(format!(
                "invalid province {other:?}: valid values are AB, BC, MB, NB, NL, NS, NT, NU, ON, PE, QC, SK, YT, OutsideCanada"
            ))),
        }
    }
}

/// Pay periods per year (P). Spec §21.10 — 53-week and 27-biweekly years are real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct PayPeriod(u16);

impl PayPeriod {
    /// Legal CRA P values (T4127 / T4032).
    pub const LEGAL: &'static [u16] = &[1, 2, 4, 10, 12, 13, 22, 24, 26, 27, 52, 53, 240, 2000];

    /// Construct from a raw period count.
    pub fn new(value: u16) -> Result<Self, RequestError> {
        if Self::LEGAL.contains(&value) {
            Ok(Self(value))
        } else {
            Err(RequestError::InvalidPayPeriod { got: value })
        }
    }

    /// Periods per year.
    pub fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for PayPeriod {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u16::deserialize(deserializer)?;
        PayPeriod::new(value).map_err(de::Error::custom)
    }
}

/// TD1 claim code 0..=10, or E (exempt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClaimCode {
    Code(u8),
    E,
}

impl Serialize for ClaimCode {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Code(n) => serializer.serialize_u8(*n),
            Self::E => serializer.serialize_str("E"),
        }
    }
}

impl<'de> Deserialize<'de> for ClaimCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ClaimCode;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("claim code 0..=10 or E")
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<ClaimCode, E> {
                if v <= 10 {
                    Ok(ClaimCode::Code(u8::try_from(v).map_err(E::custom)?))
                } else {
                    Err(E::custom(format!(
                        "invalid claim_code {v}: use 0..=10 or E"
                    )))
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<ClaimCode, E> {
                if v == "E" || v == "e" {
                    return Ok(ClaimCode::E);
                }
                if let Ok(n) = v.parse::<u8>() {
                    if n <= 10 {
                        return Ok(ClaimCode::Code(n));
                    }
                }
                Err(E::custom(format!(
                    "invalid claim_code {v:?}: use 0..=10 or E"
                )))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// How factor K2 / K2P is computed.
///
/// Variants name the *behaviour*, not the annualizing mechanism. Wire values
/// are `pdoc_observed` (default), `t4127_literal`, and `year_to_date`.
/// `annualized` is accepted as an alias of `pdoc_observed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum K2Method {
    /// PDOC observed: `max(P×C×ratio, D×ratio)` capped at `base_max×PM/12`.
    /// Does not force `base_max` in the reaching period.
    /// Vector `on-biweekly-midyear-k2-max` (D-007 inversion).
    #[default]
    #[serde(alias = "annualized")]
    PdocObserved,
    /// T4127 Chapter 3, factor K2: force `base_max` “in that pay period”
    /// when year-to-date first reaches the annual CPP maximum.
    T4127Literal,
    /// Year-to-date projection: `D + PR×C` and `D1 + PR×EI`, each capped
    /// (T4127 Chapter 4 YTD form).
    YearToDate,
}

/// Period-tax rounding when T4127 decimal half-up and live PDOC diverge.
///
/// Wire: `t4127` (default), `pdoc`. See [`docs/ADR-003-rounding-compat.md`].
/// `pdoc` deserializes but [`crate::calculate`] returns
/// [`RequestError::RoundingCompatPdocNotImplemented`] until finding 002 closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoundingCompat {
    /// Exact decimal half-up (`round_tax_to_cent`) on period lines.
    #[default]
    T4127,
    /// Reserved: match live PDOC period lines once finding 002 names the midpoint condition.
    /// Not implemented — calculate must not silently alias [`Self::T4127`].
    Pdoc,
}

fn default_option1() -> CalculationOption {
    CalculationOption::Option1
}

fn default_cpp_months() -> u8 {
    12
}

/// Full §9.3 request. Optional fields cover Option 2, commission, Quebec transfer,
/// and CPP-age paths even when M1 does not read them — the wire shape is frozen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Pay date / rules as-of. Required in core — no clock default.
    pub as_of: CalendarDate,
    pub province: Province,
    pub pay_period: PayPeriod,
    pub gross_pay: Money,

    #[serde(default = "default_option1")]
    pub calculation_option: CalculationOption,

    /// Defaults to [`Self::gross_pay`] when absent (see [`Self::pensionable_earnings`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pensionable_earnings: Option<Money>,
    /// Defaults to [`Self::gross_pay`] when absent (see [`Self::insurable_earnings`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insurable_earnings: Option<Money>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub federal_claim_code: Option<ClaimCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provincial_claim_code: Option<ClaimCode>,
    /// Direct TC override. Mutually exclusive with [`Self::federal_claim_code`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub federal_tc: Option<Money>,
    /// Direct TCP override. Mutually exclusive with [`Self::provincial_claim_code`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provincial_tcp: Option<Money>,

    /// CPP contributory months (PM). Defaults to 12.
    #[serde(default = "default_cpp_months")]
    pub cpp_months: u8,
    #[serde(default)]
    pub k2_method: K2Method,
    /// Period rounding when T4127 and PDOC diverge (ADR-003). Default `t4127`.
    #[serde(default)]
    pub rounding_compat: RoundingCompat,

    // --- Option 2 (cumulative averaging) ---------------------------------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_pensionable_earnings: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_insurable_earnings: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_cpp: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_cpp2: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_ei: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_federal_tax: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_provincial_tax: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pay_periods_elapsed: Option<u16>,

    // --- Bonus / retro / commission --------------------------------------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonus: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retroactive_pay: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_income: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_expenses: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_annual_expenses: Option<Money>,

    // --- Quebec transfer / QPP / QPIP YTD --------------------------------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_qpp: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_qpp2: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ytd_qpip: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_province: Option<Province>,

    // --- CPP age / election ----------------------------------------------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<CalendarDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpp_exempt: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpp_election_after_65: Option<bool>,

    // --- Other TD1 / formula inputs --------------------------------------
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_tax_requested: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxable_benefits: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub union_dues: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alimony: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_care_expenses: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prescribed_zone_deduction: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lcf_purchase: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lcp_purchase: Option<Money>,
    /// Eligible dependants under 19, used in T4127 factor Y (554 × count).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependants_under_19: Option<u8>,
    /// Eligible disabled dependants, used in T4127 factor Y.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependants_disabled: Option<u8>,
}

impl Request {
    /// Parse and validate a §9.3 request from JSON.
    pub fn from_json(json: &str) -> Result<Self, RequestError> {
        let mut deserializer = serde_json::Deserializer::from_str(json);
        let req: Request = serde_path_to_error::deserialize(&mut deserializer).map_err(|err| {
            RequestError::Serde {
                message: err.to_string(),
            }
        })?;
        req.validate()?;
        Ok(req)
    }

    fn validate(&self) -> Result<(), RequestError> {
        if self.federal_claim_code.is_some() && self.federal_tc.is_some() {
            return Err(RequestError::AmbiguousFederalClaim);
        }
        if self.provincial_claim_code.is_some() && self.provincial_tcp.is_some() {
            return Err(RequestError::AmbiguousProvincialClaim);
        }
        let _ = Province::ALL;
        Ok(())
    }

    /// Pensionable earnings for the period; defaults to gross pay.
    pub fn pensionable_earnings(&self) -> &Money {
        self.pensionable_earnings
            .as_ref()
            .unwrap_or(&self.gross_pay)
    }

    /// Insurable earnings for the period; defaults to gross pay.
    pub fn insurable_earnings(&self) -> &Money {
        self.insurable_earnings.as_ref().unwrap_or(&self.gross_pay)
    }
}
