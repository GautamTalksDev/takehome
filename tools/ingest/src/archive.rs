//! Recorded sha256 is checked before any CSV is parsed.

use crate::error::IngestError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Complete reprint vs mid-year overlay. Inferred only from this field, never
/// from Table 8.1 row count (CRA reprints unchanged jurisdictions in a delta).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EditionKind {
    #[default]
    Complete,
    Delta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveMeta {
    pub edition: String,
    pub effective_from: String,
    pub published_by: String,
    pub source_document: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub files: Vec<ArchiveFile>,
    /// Explicit CRA marker that this edition is a delta. Never inferred.
    #[serde(default)]
    pub edition_kind: EditionKind,
    /// Verbatim T4127 sentence that identifies a delta edition (spec §21.9).
    #[serde(default)]
    pub delta_marker: Option<String>,
    /// Jurisdictions this delta actually changes. Other Table 8.1 rows are reprints.
    #[serde(default)]
    pub delta_jurisdictions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveFile {
    pub name: String,
    pub sha256: String,
}

/// Read `archive.json` and verify every listed file's sha256 before parse.
pub fn verify_archive(dir: &Path) -> Result<ArchiveMeta, IngestError> {
    let meta_path = dir.join("archive.json");
    let raw = fs::read_to_string(&meta_path)
        .map_err(|e| IngestError::archive(&meta_path, format!("read failed: {e}")))?;
    let meta: ArchiveMeta = serde_json::from_str(&raw)
        .map_err(|e| IngestError::archive(&meta_path, format!("invalid archive.json: {e}")))?;
    if meta.files.is_empty() {
        return Err(IngestError::archive(&meta_path, "no files listed"));
    }
    for file in &meta.files {
        let path = dir.join(&file.name);
        let bytes = fs::read(&path)
            .map_err(|e| IngestError::archive(&path, format!("read failed: {e}")))?;
        let got = sha256_hex(&bytes);
        if !got.eq_ignore_ascii_case(&file.sha256) {
            return Err(IngestError::archive(
                &path,
                format!("sha256 mismatch: recorded {} got {got}", file.sha256),
            ));
        }
    }
    validate_edition_kind(&meta_path, &meta)?;
    Ok(meta)
}

fn validate_edition_kind(meta_path: &Path, meta: &ArchiveMeta) -> Result<(), IngestError> {
    match meta.edition_kind {
        EditionKind::Complete => Ok(()),
        EditionKind::Delta => {
            let marker = meta.delta_marker.as_deref().map(str::trim).unwrap_or("");
            if marker.is_empty() {
                return Err(IngestError::archive(
                    meta_path,
                    "delta edition requires an explicit delta_marker (not a row count)",
                ));
            }
            if meta.delta_jurisdictions.is_empty() {
                return Err(IngestError::archive(
                    meta_path,
                    "delta edition requires delta_jurisdictions",
                ));
            }
            Ok(())
        }
    }
}

impl ArchiveMeta {
    pub fn is_delta(&self) -> bool {
        self.edition_kind == EditionKind::Delta
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(char::from(nibble(byte >> 4)));
        out.push(char::from(nibble(byte & 0x0F)));
    }
    out
}

fn nibble(value: u8) -> u8 {
    if value < 10 {
        b'0' + value
    } else {
        b'a' + (value - 10)
    }
}

pub fn read_verified(dir: &Path, name: &str) -> Result<(PathBuf, Vec<u8>), IngestError> {
    let path = dir.join(name);
    let bytes =
        fs::read(&path).map_err(|e| IngestError::archive(&path, format!("read failed: {e}")))?;
    Ok((path, bytes))
}
