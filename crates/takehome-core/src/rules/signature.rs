//! Ed25519 signatures over embedded T4127 rule-set JSON (spec §M5 self-host).
//!
//! `takehome-core` stays IO-free: the verifying key and detached signatures
//! are `include_str!`. A self-host that swaps a JSON file without resigning
//! fails at load.

use ed25519_compact::{PublicKey, Signature};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use thiserror::Error;

const SIGNATURES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/rules/signatures.json"
));

#[cfg(test)]
mod tests {
    use super::{signed_message, verify_rules_file, SignatureError};
    use ed25519_compact::{KeyPair, Seed};
    use std::fs;
    use std::path::PathBuf;

    fn rules_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/rules")
    }

    /// 45. A keypair signs a blob; a flipped byte fails verification.
    #[test]
    fn ed25519_rejects_a_tampered_message() {
        let seed = Seed::new([7u8; 32]);
        let kp = KeyPair::from_seed(seed);
        let msg = b"2026-01-01/federal.json\0{\"k\":1}";
        let sig = kp.sk.sign(msg, None);
        assert!(kp.pk.verify(msg, &sig).is_ok());
        let mut bad = msg.to_vec();
        bad[0] ^= 1;
        assert!(kp.pk.verify(&bad, &sig).is_err());
    }

    /// 45. Every embedded rule-set file verifies against the committed signatures.
    #[test]
    fn every_embedded_rule_file_verifies() {
        let bundle: serde_json::Value = serde_json::from_str(super::SIGNATURES_JSON).unwrap();
        let files = bundle["files"].as_object().expect("files map");
        assert!(files.len() >= 30, "got {}", files.len());
        for path in files.keys() {
            let bytes = fs::read(rules_root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
            verify_rules_file(path, &bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
    }

    /// Vendored JSON must stay byte-identical to `data/rules/` (sdist rebuild).
    #[test]
    fn vendored_rules_are_byte_identical_to_data_rules() {
        let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/rules");
        if !canonical.join("signatures.json").exists() {
            return;
        }
        fn collect(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
            let mut out = Vec::new();
            fn walk(
                dir: &std::path::Path,
                root: &std::path::Path,
                out: &mut Vec<(String, Vec<u8>)>,
            ) {
                for entry in fs::read_dir(dir).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, root, out);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        let rel = path
                            .strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/");
                        out.push((rel, fs::read(&path).unwrap()));
                    }
                }
            }
            walk(root, root, &mut out);
            out.sort_by(|a, b| a.0.cmp(&b.0));
            out
        }
        let src = collect(&canonical);
        let dst = collect(&rules_root());
        assert_eq!(src, dst, "vendor/rules drifted from data/rules");
        assert!(!rules_root().join("signing.seed").exists());
    }

    /// 45. A lost or swapped rule file is not loaded as the published edition.
    #[test]
    fn a_flipped_byte_in_an_embedded_file_fails() {
        let path = "2026-01-01/federal.json";
        let mut bytes = fs::read(rules_root().join(path)).unwrap();
        bytes[0] ^= 0xff;
        match verify_rules_file(path, &bytes) {
            Err(SignatureError::Invalid(got)) => assert_eq!(got, path),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn unknown_path_is_a_missing_signature() {
        match verify_rules_file("nope.json", b"{}") {
            Err(SignatureError::Missing(got)) => assert_eq!(got, "nope.json"),
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn signed_message_binds_the_path() {
        let a = signed_message("a.json", b"x");
        let b = signed_message("b.json", b"x");
        assert_ne!(a, b);
        assert_eq!(a[a.len() - 1], b'x');
    }
}

/// Failure verifying an embedded rule-set file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SignatureError {
    #[error("rules signature missing for {0}")]
    Missing(String),
    #[error("rules signature invalid for {0}")]
    Invalid(String),
    #[error("rules signing key is not valid")]
    Key,
}

#[derive(Debug, Deserialize)]
struct SignatureBundle {
    algorithm: String,
    public_key: String,
    files: BTreeMap<String, String>,
}

fn bundle() -> Result<&'static SignatureBundle, SignatureError> {
    static BUNDLE: OnceLock<SignatureBundle> = OnceLock::new();
    if let Some(parsed) = BUNDLE.get() {
        return Ok(parsed);
    }
    let parsed = parse_bundle(SIGNATURES_JSON)?;
    Ok(BUNDLE.get_or_init(|| parsed))
}

fn parse_bundle(json: &str) -> Result<SignatureBundle, SignatureError> {
    let parsed: SignatureBundle = serde_json::from_str(json).map_err(|_| SignatureError::Key)?;
    if parsed.algorithm != "ed25519" {
        return Err(SignatureError::Key);
    }
    if parsed.public_key.len() != 64 {
        return Err(SignatureError::Key);
    }
    Ok(parsed)
}

fn hex_val(byte: u8) -> Result<u8, SignatureError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(SignatureError::Key),
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, SignatureError> {
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(SignatureError::Key);
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_val(bytes[i])?;
        let lo = hex_val(bytes[i + 1])?;
        out.push(hi.wrapping_shl(4) | lo);
        i += 2;
    }
    Ok(out)
}

fn signed_message(path: &str, bytes: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(path.len() + 1 + bytes.len());
    msg.extend_from_slice(path.as_bytes());
    msg.push(0);
    msg.extend_from_slice(bytes);
    msg
}

/// Verify an embedded T4127 rule-set file against the committed Ed25519
/// signature (spec §M5 self-host). The engine refuses to assemble a
/// registry from bytes that do not match.
pub fn verify_rules_file(path: &str, bytes: &[u8]) -> Result<(), SignatureError> {
    let bundle = bundle()?;
    let sig_hex = bundle
        .files
        .get(path)
        .ok_or_else(|| SignatureError::Missing(path.to_string()))?;
    let pk_bytes = hex_decode(&bundle.public_key)?;
    let sig_bytes = hex_decode(sig_hex)?;
    let pk = PublicKey::from_slice(&pk_bytes).map_err(|_| SignatureError::Key)?;
    let sig =
        Signature::from_slice(&sig_bytes).map_err(|_| SignatureError::Invalid(path.to_string()))?;
    let msg = signed_message(path, bytes);
    pk.verify(&msg, &sig)
        .map_err(|_| SignatureError::Invalid(path.to_string()))
}
