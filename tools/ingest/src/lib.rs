//! CRA T4127 CSV ingest. Moves text. Never parses a URL. Never parses a number.

pub mod archive;
pub mod emit;
pub mod encoding;
pub mod error;
pub mod lexical;
pub mod provinces;
pub mod tables;
pub mod tokens;

use std::path::Path;

pub use archive::{verify_archive, ArchiveMeta, EditionKind};
pub use emit::jurisdiction_json;
pub use encoding::{decode_bytes, Decoded, EncodingKind};
pub use error::IngestError;
pub use lexical::{pad_fraction, strip_thousands};
pub use provinces::{
    cra_code_to_jurisdiction, filename_to_jurisdiction, JURISDICTION_TO_CRA, TABLE_8_1, THIRTEEN,
};
pub use tables::{
    csv_rows, parse_bracket_table, parse_other_amounts, BasicCell, Bracket, OtherAmounts,
    ParsedBundle,
};
pub use tokens::{special_token, SpecialToken};

use crate::emit::{write_rule_directory, Overlay};

/// Options for overlaying a delta edition onto a complete base rule set.
#[derive(Debug, Clone, Copy)]
pub struct IngestOptions<'a> {
    /// Previously ingested complete rule directory (January, for a July delta).
    pub base_rules_dir: Option<&'a Path>,
    /// When false, a delta edition is not filled in from the base set.
    /// That is the failure mode spec §21.9 requires a hard error for.
    pub overlay: bool,
}

impl Default for IngestOptions<'static> {
    fn default() -> Self {
        Self {
            base_rules_dir: None,
            overlay: true,
        }
    }
}

/// Ingest an already-archived edition directory into `out_dir`.
///
/// Hashes in `archive.json` are checked before any CSV is parsed.
pub fn ingest_archive(archive_dir: &Path, out_dir: &Path) -> Result<IngestReport, IngestError> {
    ingest_archive_with(archive_dir, out_dir, IngestOptions::default())
}

/// Ingest with explicit overlay controls (delta editions).
pub fn ingest_archive_with(
    archive_dir: &Path,
    out_dir: &Path,
    options: IngestOptions<'_>,
) -> Result<IngestReport, IngestError> {
    let meta = verify_archive(archive_dir)?;
    let bundle = ParsedBundle::from_archive(archive_dir, &meta)?;
    let written = write_rule_directory(
        out_dir,
        &meta,
        &bundle,
        Overlay {
            base_rules_dir: options.base_rules_dir,
            enabled: options.overlay,
        },
    )?;
    Ok(IngestReport {
        edition: meta.edition,
        effective_from: meta.effective_from,
        files_written: written,
    })
}

/// Summary of a successful ingest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestReport {
    pub edition: String,
    pub effective_from: String,
    pub files_written: Vec<String>,
}

/// Git sha of this ingest crate, baked at compile time.
pub fn ingest_git_sha() -> &'static str {
    env!("TAKEHOME_INGEST_GIT_SHA")
}

/// Crate version.
pub fn tool_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
