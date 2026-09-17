//! Emit `engine_build_sha256` for Response provenance (M1 step 12).
//!
//! Hashes the engine source tree under `src/` (deterministic, sorted paths).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest_dir.join("src");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");

    let digest = hash_tree(&src).unwrap_or_else(|e| {
        eprintln!("cargo:warning=engine_build_sha256 hash failed: {e}");
        "HASH_FAILED".to_string()
    });
    println!("cargo:rustc-env=TAKEHOME_ENGINE_BUILD_SHA256={digest}");
}

fn hash_tree(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_rs_files(root, &mut files)?;
    files.sort();
    // Prefer system sha256sum for a stable hex digest without extra crates.
    let mut cmd = Command::new("sha256sum");
    for f in &files {
        cmd.arg(f);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    // Aggregate: hash the concatenation of "hex  path\n" lines (sorted).
    let listing = String::from_utf8_lossy(&out.stdout);
    let hasher_input = listing.into_owned().into_bytes();
    // Final digest of the listing itself for a single tree id.
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .ok_or("stdin")?
            .write_all(&hasher_input)
            .map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let line = String::from_utf8_lossy(&out.stdout);
    let hex = line
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty sha256sum".to_string())?
        .to_string();
    let _ = hasher_input;
    Ok(hex)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}
