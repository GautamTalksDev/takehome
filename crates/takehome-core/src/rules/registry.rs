//! Effective-date registry for T4127 rule sets.
//!
//! Intervals are half-open `[effective_from, effective_to)`. `effective_to: None`
//! means the set is open-ended. The registry does not read a clock; `as_of`
//! arrives on the request.

use crate::rules::schema::{CalendarDate, RuleSet};
use thiserror::Error;

#[cfg(test)]
mod tests {
    use super::{Registry, RuleError};
    use crate::rules::schema::{
        CalendarDate, CppParams, EiParams, QpipParams, RuleSet, RuleSetStatus,
    };
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn date(s: &str) -> CalendarDate {
        s.parse().expect("fixture date")
    }

    fn set(version: &str, from: &str, to: Option<&str>) -> RuleSet {
        RuleSet {
            rule_set_version: version.to_string(),
            effective_from: date(from),
            effective_to: to.map(date),
            published_by: "CRA".to_string(),
            source_document: "T4127".to_string(),
            source_url: "https://www.canada.ca/".to_string(),
            retrieved_at: "2026-01-15T00:00:00Z".to_string(),
            source_sha256: "aa".repeat(32),
            status: RuleSetStatus::Enacted,
            announcement_source: None,
            announcement_date: None,
            jurisdictions: BTreeMap::new(),
            cpp: stub_cpp(),
            ei: stub_ei(),
            qpip: stub_qpip(),
        }
    }

    fn stub_cpp() -> CppParams {
        CppParams {
            ympe: crate::decimal::Money::parse("74600.00").unwrap(),
            yampe: crate::decimal::Money::parse("85000.00").unwrap(),
            basic_exemption: crate::decimal::Money::parse("3500.00").unwrap(),
            total_rate: crate::decimal::Rate::parse("0.0595").unwrap(),
            total_max: crate::decimal::Money::parse("4230.45").unwrap(),
            base_rate: crate::decimal::Rate::parse("0.0495").unwrap(),
            base_max: crate::decimal::Money::parse("3519.45").unwrap(),
            first_additional_rate: crate::decimal::Rate::parse("0.0100").unwrap(),
            first_additional_max: crate::decimal::Money::parse("711.00").unwrap(),
            second_additional_rate: crate::decimal::Rate::parse("0.0400").unwrap(),
            second_additional_max: crate::decimal::Money::parse("416.00").unwrap(),
        }
    }

    fn stub_ei() -> EiParams {
        EiParams {
            max_insurable: crate::decimal::Money::parse("68900.00").unwrap(),
            employee_rate: crate::decimal::Rate::parse("0.0163").unwrap(),
            employee_max: crate::decimal::Money::parse("1123.07").unwrap(),
            employer_rate: crate::decimal::Rate::parse("0.02282").unwrap(),
            employer_max: crate::decimal::Money::parse("1572.30").unwrap(),
        }
    }

    fn stub_qpip() -> QpipParams {
        QpipParams {
            max_insurable: crate::decimal::Money::parse("103000.00").unwrap(),
            employee_rate: crate::decimal::Rate::parse("0.00430").unwrap(),
            employee_max: crate::decimal::Money::parse("442.90").unwrap(),
            employer_rate: crate::decimal::Rate::parse("0.00602").unwrap(),
            employer_max: crate::decimal::Money::parse("620.06").unwrap(),
        }
    }

    fn jan_and_july() -> Registry {
        Registry::load(vec![
            set("2026-01-01", "2026-01-01", Some("2026-07-01")),
            set("2026-07-01", "2026-07-01", None),
        ])
        .expect("contiguous jan/july")
    }

    /// Test 13: Intervals are HALF-OPEN [from, to). 2026-07-01 resolves to the July
    /// set, not the January one. This one-day boundary is the whole feature.
    #[test]
    fn half_open_boundary_july_first_is_july_set() {
        let registry = jan_and_july();
        let jan = registry.resolve(date("2026-06-30")).expect("june 30");
        let july = registry.resolve(date("2026-07-01")).expect("july 1");
        assert_eq!(jan.rule_set_version, "2026-01-01");
        assert_eq!(july.rule_set_version, "2026-07-01");
        assert_ne!(jan.rule_set_version, july.rule_set_version);
    }

    /// Test 14: as_of before the earliest set returns DateBeforeCoverage. It does
    /// NOT return the nearest set. Guessing is how a customer silently gets
    /// 2025 rates in 2027.
    #[test]
    fn before_earliest_is_date_before_coverage_not_nearest() {
        let registry = jan_and_july();
        let requested = date("2025-12-31");
        let err = registry.resolve(requested).expect_err("must not guess");
        match err {
            RuleError::DateBeforeCoverage {
                requested: got,
                earliest,
                ..
            } => {
                assert_eq!(got, requested);
                assert_eq!(earliest, date("2026-01-01"));
            }
            other => panic!("expected DateBeforeCoverage, got {other:?}"),
        }
    }

    /// 15. as_of after the latest set's effective_to returns DateAfterCoverage.
    #[test]
    fn after_latest_effective_to_is_date_after_coverage() {
        let registry = Registry::load(vec![set("2026-01-01", "2026-01-01", Some("2026-07-01"))])
            .expect("single closed set");
        let requested = date("2026-07-01");
        let err = registry.resolve(requested).expect_err("outside window");
        match err {
            RuleError::DateAfterCoverage {
                requested: got,
                latest,
                ..
            } => {
                assert_eq!(got, requested);
                assert_eq!(latest, date("2026-07-01"));
            }
            other => panic!("expected DateAfterCoverage, got {other:?}"),
        }
    }

    /// Test 16: A set with effective_to: None covers everything from its start
    /// forward. Only the newest set may have None; any other is a load error.
    #[test]
    fn open_ended_newest_covers_forward_older_open_ended_is_load_error() {
        let registry = jan_and_july();
        assert_eq!(
            registry
                .resolve(date("2027-03-15"))
                .expect("open-ended")
                .rule_set_version,
            "2026-07-01"
        );

        let err = Registry::load(vec![
            set("2026-01-01", "2026-01-01", None),
            set("2026-07-01", "2026-07-01", None),
        ])
        .expect_err("only newest may be open-ended");
        match err {
            RuleError::OpenEndedNotNewest { version, .. } => {
                assert_eq!(version, "2026-01-01");
            }
            other => panic!("expected OpenEndedNotNewest, got {other:?}"),
        }
    }

    /// 17. Overlapping intervals across two sets is a load error naming both versions.
    #[test]
    fn overlapping_intervals_are_a_load_error_naming_both_versions() {
        let err = Registry::load(vec![
            set("2026-01-01", "2026-01-01", Some("2026-08-01")),
            set("2026-07-01", "2026-07-01", None),
        ])
        .expect_err("overlap");
        let msg = err.to_string();
        assert!(
            msg.contains("2026-01-01") && msg.contains("2026-07-01"),
            "overlap error must name both versions, got {msg}"
        );
        match err {
            RuleError::OverlappingIntervals { first, second, .. } => {
                assert_eq!(first, "2026-01-01");
                assert_eq!(second, "2026-07-01");
            }
            other => panic!("expected OverlappingIntervals, got {other:?}"),
        }
    }

    /// 18. A gap between sets is a load error. Coverage must be contiguous.
    #[test]
    fn gap_between_sets_is_a_load_error() {
        let err = Registry::load(vec![
            set("2026-01-01", "2026-01-01", Some("2026-07-01")),
            set("2026-08-01", "2026-08-01", None),
        ])
        .expect_err("gap");
        match err {
            RuleError::CoverageGap {
                first,
                second,
                first_to,
                second_from,
                ..
            } => {
                assert_eq!(first, "2026-01-01");
                assert_eq!(second, "2026-08-01");
                assert_eq!(first_to, date("2026-07-01"));
                assert_eq!(second_from, date("2026-08-01"));
            }
            other => panic!("expected CoverageGap, got {other:?}"),
        }
    }

    /// 19. resolve is pure: same as_of always gives the same version, no clock read.
    #[test]
    fn resolve_is_pure_same_as_of_same_version() {
        let registry = jan_and_july();
        let as_of = date("2026-03-15");
        let a = registry.resolve(as_of).expect("first");
        let b = registry.resolve(as_of).expect("second");
        assert_eq!(a.rule_set_version, b.rule_set_version);
        assert!(std::ptr::eq(a, b));
    }

    fn date_2026_ordinal(ordinal0: u32) -> CalendarDate {
        let dims = [31u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut rem = ordinal0;
        for (i, dim) in dims.iter().enumerate() {
            if rem < *dim {
                let month = u8::try_from(i + 1).expect("month");
                let day = u8::try_from(rem + 1).expect("day");
                return CalendarDate::new(2026, month, day).expect("2026 ordinal");
            }
            rem -= *dim;
        }
        panic!("ordinal out of 2026")
    }

    // 20. Property: for any two dates a < b, resolve(a).effective_from <=
    // resolve(b).effective_from.
    proptest! {
        #[test]
        fn resolve_effective_from_is_monotone(ord_a in 0u32..=364, ord_b in 0u32..=364) {
            prop_assume!(ord_a < ord_b);
            let registry = jan_and_july();
            let a = date_2026_ordinal(ord_a);
            let b = date_2026_ordinal(ord_b);
            prop_assume!(a < b);
            let sa = registry.resolve(a).expect("a in coverage");
            let sb = registry.resolve(b).expect("b in coverage");
            prop_assert!(sa.effective_from <= sb.effective_from);
        }
    }
}

/// Sorted, contiguous, half-open coverage over T4127 rule sets.
#[derive(Debug)]
pub struct Registry {
    sets: Vec<RuleSet>,
}

/// Actionable registry failure. Variants carry the requested date (when
/// resolving) and the coverage window so an API body needs no support ticket.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleError {
    /// `as_of` is before the first `effective_from`. Never guess a nearest set.
    #[error(
        "requested date {requested} is before coverage starting {earliest} (window [{earliest}, {latest:?}))"
    )]
    DateBeforeCoverage {
        requested: CalendarDate,
        earliest: CalendarDate,
        latest: Option<CalendarDate>,
    },
    /// `as_of` is on or after the last set's exclusive `effective_to`.
    #[error(
        "requested date {requested} is after coverage ending {latest} (window [{earliest}, {latest}))"
    )]
    DateAfterCoverage {
        requested: CalendarDate,
        earliest: CalendarDate,
        latest: CalendarDate,
    },
    /// `load` was given no rule sets.
    #[error("no rule sets provided; coverage window is empty")]
    EmptyRegistry,
    /// Named edition is not in the registry. Never nearest-match another year.
    #[error("unknown rule set version {version}")]
    UnknownVersion { version: String },
    /// Only the newest set may have `effective_to: None`.
    #[error(
        "rule set {version} has open-ended effective_to but is not the newest (starts {effective_from}; window [{coverage_from}, {coverage_to:?}))"
    )]
    OpenEndedNotNewest {
        version: String,
        effective_from: CalendarDate,
        coverage_from: CalendarDate,
        coverage_to: Option<CalendarDate>,
    },
    /// Two sets claim the same instant. Names both versions.
    #[error(
        "rule sets {first} and {second} overlap: [{first_from}, {first_to:?}) vs [{second_from}, {second_to:?})"
    )]
    OverlappingIntervals {
        first: String,
        second: String,
        first_from: CalendarDate,
        first_to: Option<CalendarDate>,
        second_from: CalendarDate,
        second_to: Option<CalendarDate>,
    },
    /// Coverage is not contiguous.
    #[error(
        "coverage gap between {first} ending {first_to} and {second} starting {second_from} (window [{coverage_from}, {coverage_to:?}))"
    )]
    CoverageGap {
        first: String,
        second: String,
        first_to: CalendarDate,
        second_from: CalendarDate,
        coverage_from: CalendarDate,
        coverage_to: Option<CalendarDate>,
    },
    /// A single set has `effective_to <= effective_from`.
    #[error("rule set {version} has inverted interval [{effective_from}, {effective_to})")]
    InvertedInterval {
        version: String,
        effective_from: CalendarDate,
        effective_to: CalendarDate,
    },
}

impl Registry {
    /// Build a registry. Sets are sorted by `effective_from`; intervals must
    /// be contiguous and half-open (T4127 effective-date tables).
    pub fn load(mut sets: Vec<RuleSet>) -> Result<Registry, RuleError> {
        if sets.is_empty() {
            return Err(RuleError::EmptyRegistry);
        }
        sets.sort_by_key(|set| set.effective_from);

        let coverage_from = sets[0].effective_from;
        let coverage_to = sets[sets.len() - 1].effective_to;

        for (index, set) in sets.iter().enumerate() {
            if let Some(to) = set.effective_to {
                if to <= set.effective_from {
                    return Err(RuleError::InvertedInterval {
                        version: set.rule_set_version.clone(),
                        effective_from: set.effective_from,
                        effective_to: to,
                    });
                }
            } else if index + 1 != sets.len() {
                return Err(RuleError::OpenEndedNotNewest {
                    version: set.rule_set_version.clone(),
                    effective_from: set.effective_from,
                    coverage_from,
                    coverage_to,
                });
            }
        }

        for pair in sets.windows(2) {
            let first = &pair[0];
            let second = &pair[1];
            let first_to = first
                .effective_to
                .expect("non-newest open-ended already rejected");
            if first_to > second.effective_from {
                return Err(RuleError::OverlappingIntervals {
                    first: first.rule_set_version.clone(),
                    second: second.rule_set_version.clone(),
                    first_from: first.effective_from,
                    first_to: first.effective_to,
                    second_from: second.effective_from,
                    second_to: second.effective_to,
                });
            }
            if first_to < second.effective_from {
                return Err(RuleError::CoverageGap {
                    first: first.rule_set_version.clone(),
                    second: second.rule_set_version.clone(),
                    first_to,
                    second_from: second.effective_from,
                    coverage_from,
                    coverage_to,
                });
            }
        }

        Ok(Registry { sets })
    }

    /// Select the rule set whose half-open interval contains `as_of`.
    ///
    /// Pure: same `as_of` always yields the same version. No clock is read
    /// (T4127 effective-date selection).
    pub fn resolve(&self, as_of: CalendarDate) -> Result<&RuleSet, RuleError> {
        let earliest = self.sets[0].effective_from;
        let latest = self.sets[self.sets.len() - 1].effective_to;

        if as_of < earliest {
            return Err(RuleError::DateBeforeCoverage {
                requested: as_of,
                earliest,
                latest,
            });
        }

        for set in &self.sets {
            let in_from = as_of >= set.effective_from;
            let in_to = set.effective_to.map(|to| as_of < to).unwrap_or(true);
            if in_from && in_to {
                return Ok(set);
            }
        }

        let latest = latest.expect("open-ended newest would have matched");
        Err(RuleError::DateAfterCoverage {
            requested: as_of,
            earliest,
            latest,
        })
    }

    /// `(version, effective_from, effective_to)` in coverage order.
    pub fn versions(&self) -> Vec<(&str, CalendarDate, Option<CalendarDate>)> {
        self.sets
            .iter()
            .map(|set| {
                (
                    set.rule_set_version.as_str(),
                    set.effective_from,
                    set.effective_to,
                )
            })
            .collect()
    }

    /// Exact edition lookup. Unknown names are `None`, never a neighbour.
    pub fn get(&self, version: &str) -> Option<&RuleSet> {
        self.sets.iter().find(|set| set.rule_set_version == version)
    }
}
