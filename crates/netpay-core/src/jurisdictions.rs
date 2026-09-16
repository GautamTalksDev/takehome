//! Product listing for `GET /v1/jurisdictions`.
//!
//! Quebec is named here as unsupported so a missing calculator cannot be
//! mistaken for a successful federal-only result.

use crate::request::Province;
use serde::Serialize;

/// Reason returned on a Quebec request and published on the jurisdictions list.
pub const QUEBEC_UNSUPPORTED_REASON: &str = "Quebec uses Revenu Québec formulas (QPP/QPIP), not T4127 Chapter 4 provincial tax. This engine does not calculate Quebec and will not return a federal-only answer with T2 = 0. Roadmap: docs/jurisdictions.md.";

/// Machine-readable catalog served by `GET /v1/jurisdictions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JurisdictionListing {
    pub jurisdictions: Vec<JurisdictionStatus>,
}

/// One province, territory, or Outside Canada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JurisdictionStatus {
    pub code: String,
    pub name: String,
    pub supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Every [`Province`] value, with Quebec marked unsupported.
pub fn list_jurisdictions() -> JurisdictionListing {
    JurisdictionListing {
        jurisdictions: Province::all().iter().copied().map(status).collect(),
    }
}

/// Pretty JSON for the CLI, wasm export, and `/v1/jurisdictions`.
pub fn jurisdictions_json() -> String {
    serde_json::to_string_pretty(&list_jurisdictions()).expect("listing is always serializable")
}

fn status(province: Province) -> JurisdictionStatus {
    match province {
        Province::Qc => JurisdictionStatus {
            code: province.as_str().to_string(),
            name: province.display_name().to_string(),
            supported: false,
            note: Some(QUEBEC_UNSUPPORTED_REASON.to_string()),
        },
        _ => JurisdictionStatus {
            code: province.as_str().to_string(),
            name: province.display_name().to_string(),
            supported: true,
            note: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{list_jurisdictions, QUEBEC_UNSUPPORTED_REASON};
    use crate::request::Province;

    #[test]
    fn quebec_is_listed_unsupported_and_names_the_roadmap() {
        let listing = list_jurisdictions();
        assert_eq!(listing.jurisdictions.len(), Province::all().len());
        let qc = listing
            .jurisdictions
            .iter()
            .find(|row| row.code == "QC")
            .expect("QC must appear on the listing");
        assert_eq!(qc.name, "Quebec");
        assert!(!qc.supported);
        let note = qc.note.as_deref().expect("QC note");
        assert!(note.contains("Quebec"));
        assert!(note.contains("docs/jurisdictions.md"));
        assert_eq!(note, QUEBEC_UNSUPPORTED_REASON);
        assert!(
            listing
                .jurisdictions
                .iter()
                .filter(|row| !row.supported)
                .count()
                == 1
        );
        assert!(listing
            .jurisdictions
            .iter()
            .any(|row| row.code == "OutsideCanada" && row.supported));
    }
}
