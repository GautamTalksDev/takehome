//! Explicit CRA code ↔ jurisdiction mapping. Unmapped codes are hard errors.

use crate::error::IngestError;

/// The thirteen provincial / territorial jurisdictions.
pub const THIRTEEN: &[&str] = &[
    "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "QC", "SK", "YT",
];

/// Table 8.1 jurisdictions a complete rule set must contain (FED + 12, no QC).
pub const TABLE_8_1: &[&str] = &[
    "FED", "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "SK", "YT",
];

/// Reverse map: canonical jurisdiction → CRA filename token.
pub const JURISDICTION_TO_CRA: &[(&str, &str)] = &[
    ("AB", "ab"),
    ("BC", "bc"),
    ("MB", "mb"),
    ("NB", "nb"),
    ("NL", "nl"),
    ("NS", "ns"),
    ("NT", "nt"),
    ("NU", "nv"),
    ("ON", "on"),
    ("PE", "pei"),
    ("QC", "qc"),
    ("SK", "sk"),
    ("YT", "yt"),
    ("FED", "fd"),
];

/// CRA source token → canonical jurisdiction code.
///
/// `nv` is Nunavut (not `nu`). `pei` is Prince Edward Island (not `pe`).
/// `nu` and `pe` are unmapped and error. Never fall through to the input.
pub fn cra_code_to_jurisdiction(raw: &str) -> Result<&'static str, IngestError> {
    let key = raw.trim();
    match key {
        "ab" | "AB" => Ok("AB"),
        "bc" | "BC" => Ok("BC"),
        "mb" | "MB" => Ok("MB"),
        "nb" | "NB" => Ok("NB"),
        "nl" | "NL" => Ok("NL"),
        "ns" | "NS" => Ok("NS"),
        "nt" | "NT" => Ok("NT"),
        "nv" | "NV" | "NU" => Ok("NU"),
        "on" | "ON" => Ok("ON"),
        "pei" | "PEI" | "PE" => Ok("PE"),
        "qc" | "QC" => Ok("QC"),
        "sk" | "SK" => Ok("SK"),
        "yt" | "YT" => Ok("YT"),
        "fd" | "FD" | "FED" | "Federal" => Ok("FED"),
        "Outside Canada" | "OUTSIDE_CANADA" | "OutsideCanada" => Ok("OUTSIDE_CANADA"),
        other => Err(IngestError::message(format!(
            "unmapped CRA jurisdiction code {other:?}; never fall through"
        ))),
    }
}

/// Map a claim-code filename stem (`cc-nv-01-26e`) to a jurisdiction.
pub fn filename_to_jurisdiction(filename: &str) -> Result<&'static str, IngestError> {
    let stem = filename
        .strip_prefix("cc-")
        .and_then(|s| s.split('-').next())
        .ok_or_else(|| IngestError::message(format!("not a claim-code filename: {filename}")))?;
    cra_code_to_jurisdiction(stem)
}
