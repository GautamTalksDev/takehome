//! Emit `engine_build_sha256` for Response provenance (M1 step 12).
//!
//! Hashes the engine source tree under `src/` (deterministic, sorted paths).
//! Paths in the digest are relative to `src/` so the digest does not change
//! when the repo is checked out under a different absolute prefix (WSL home
//! vs GitHub Actions `/home/runner/work/...`).

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
    // Content hashes with paths relative to `root` (never absolute).
    let mut listing = String::new();
    for f in &files {
        let out = Command::new("sha256sum")
            .arg(f)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "sha256sum failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let hex = String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .ok_or_else(|| "empty sha256sum".to_string())?
            .to_string();
        let rel = f
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        listing.push_str(&hex);
        listing.push_str("  ");
        listing.push_str(&rel);
        listing.push('\n');
    }
    let hasher_input = listing.into_bytes();
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
