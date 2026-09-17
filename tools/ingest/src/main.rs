use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use takehome_ingest::{ingest_archive_with, IngestError, IngestOptions};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut archive: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut base: Option<PathBuf> = None;
    let mut overlay = true;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base" => {
                base = args.next().map(PathBuf::from);
                if base.is_none() {
                    eprintln!("usage: takehome-ingest <archive-dir> <out-dir> [--base <rules-dir>] [--no-overlay]");
                    return ExitCode::from(2);
                }
            }
            "--no-overlay" => overlay = false,
            "--help" | "-h" => {
                eprintln!("usage: takehome-ingest <archive-dir> <out-dir> [--base <rules-dir>] [--no-overlay]");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown flag {other}");
                return ExitCode::from(2);
            }
            other => {
                if archive.is_none() {
                    archive = Some(PathBuf::from(other));
                } else if out.is_none() {
                    out = Some(PathBuf::from(other));
                } else {
                    eprintln!("usage: takehome-ingest <archive-dir> <out-dir> [--base <rules-dir>] [--no-overlay]");
                    return ExitCode::from(2);
                }
            }
        }
    }
    let (Some(archive), Some(out)) = (archive, out) else {
        eprintln!(
            "usage: takehome-ingest <archive-dir> <out-dir> [--base <rules-dir>] [--no-overlay]"
        );
        return ExitCode::from(2);
    };
    match ingest_archive_with(
        &archive,
        &out,
        IngestOptions {
            base_rules_dir: base.as_deref(),
            overlay,
        },
    ) {
        Ok(report) => {
            eprintln!(
                "ingested {} → {} ({} files)",
                report.edition,
                report.effective_from,
                report.files_written.len()
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            match err {
                IngestError::Row {
                    file,
                    line,
                    content,
                    detail,
                } => eprintln!("{file} line {line}: {content}: {detail}"),
                other => eprintln!("{other}"),
            }
            ExitCode::from(1)
        }
    }
}
