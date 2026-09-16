//! Hard errors. Always name the file, line, and content when a row is involved.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IngestError {
    #[error("{0}")]
    Message(String),
    #[error("{file} line {line}: {content}: {detail}")]
    Row {
        file: String,
        line: usize,
        content: String,
        detail: String,
    },
    #[error("archive {path}: {detail}")]
    Archive { path: String, detail: String },
}

impl IngestError {
    pub fn message(detail: impl Into<String>) -> Self {
        Self::Message(detail.into())
    }

    pub fn archive(path: &std::path::Path, detail: impl Into<String>) -> Self {
        Self::Archive {
            path: path.display().to_string(),
            detail: detail.into(),
        }
    }

    pub fn row(file: &str, line: usize, content: &str, detail: impl Into<String>) -> Self {
        Self::Row {
            file: file.to_string(),
            line,
            content: content.to_string(),
            detail: detail.into(),
        }
    }

    pub fn path_message(path: PathBuf, detail: impl Into<String>) -> Self {
        Self::Message(format!("{}: {}", path.display(), detail.into()))
    }
}
