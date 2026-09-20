//! Deduction response (spec §9.3). Stable field order; money always as JSON strings.

use crate::decimal::{Money, Rate};
use crate::formulas::option2::S1;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Placeholder until CI injects the build digest in step 12.
pub const ENGINE_BUILD_SHA256: &str = env!("TAKEHOME_ENGINE_BUILD_SHA256");

#[cfg(test)]
mod tests {
    use super::{
        AnnualProjection, Breakdown, Citation, EmployeeAmounts, EmployerAmounts, Response,
        ENGINE_BUILD_SHA256,
    };
    use crate::decimal::{Money, Rate};
    use serde::Deserialize;
    use serde_json::Value;

    fn zero() -> Money {
        Money::ZERO
    }

    fn zero_rate() -> Rate {
        Rate::parse("0").unwrap()
    }

    fn sample_response() -> Response {
        Response {
            rule_set_version: "2026-01-01".to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            engine_build_sha256: ENGINE_BUILD_SHA256.to_string(),
            prorated_rules_applied: false,
            employee: EmployeeAmounts {
                federal_tax: zero(),
                provincial_tax: zero(),
                total_tax: zero(),
                cpp: zero(),
                cpp2: zero(),
                ei: zero(),
                qpip: zero(),
                total_deductions: zero(),
                net_pay: Money::parse("2500.00").unwrap(),
            },
            employer: EmployerAmounts {
                cpp: zero(),
                cpp2: zero(),
                ei: zero(),
                qpip: zero(),
            },
            annual_projection: AnnualProjection {
                taxable_income: zero(),
                federal_tax: zero(),
                provincial_tax: zero(),
                cpp: zero(),
                cpp2: zero(),
                ei: zero(),
                qpip: zero(),
            },
            breakdown: Breakdown::zeros(),
            citations: vec![Citation {
                factor: "K".to_string(),
                source_document: "T4127 Table 8.1".to_string(),
                source_url: "https://www.canada.ca/".to_string(),
            }],
            warnings: vec![],
        }
    }

    fn assert_money_fields_are_strings(value: &Value, path: &str) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    let child = format!("{path}.{k}");
                    match v {
                        Value::Number(n) => {
                            // Only non-money numerics allowed (bools already excluded).
                            // Money must never appear as a JSON number.
                            panic!("numeric JSON at {child}: {n}; money must be a quoted string");
                        }
                        _ => assert_money_fields_are_strings(v, &child),
                    }
                }
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    assert_money_fields_are_strings(item, &format!("{path}[{i}]"));
                }
            }
            _ => {}
        }
    }

    /// 29. Every response money field serialises as a quoted string.
    #[test]
    fn every_response_money_field_serialises_as_quoted_string() {
        let encoded = serde_json::to_value(sample_response()).unwrap();
        // Spot-check known money paths are strings.
        assert!(encoded["employee"]["federal_tax"].is_string());
        assert!(encoded["employee"]["net_pay"].is_string());
        assert!(encoded["employer"]["cpp"].is_string());
        assert!(encoded["annual_projection"]["taxable_income"].is_string());
        assert!(encoded["breakdown"]["A"].is_string());
        assert!(encoded["breakdown"]["K"].is_string());
        assert!(encoded["breakdown"]["T1"].is_string());
        // Rates are strings too (lexical), never IEEE numbers.
        assert!(encoded["breakdown"]["R"].is_string());
        assert!(encoded["breakdown"]["V"].is_string());
        assert!(encoded["breakdown"]["P"].is_string());
        assert!(encoded["breakdown"]["QPIP"].is_string());
        assert_money_fields_are_strings(&encoded, "$");
    }

    // Test 30: response serialises with keys in a stable order.
    // Byte-identical output for identical input is an M2 invariant.
    #[test]
    fn response_serialisation_is_byte_stable() {
        let a = serde_json::to_string(&sample_response()).unwrap();
        let b = serde_json::to_string(&sample_response()).unwrap();
        assert_eq!(a, b);
        // Struct field order: top-level keys appear in declaration order.
        let positions = [
            "rule_set_version",
            "engine_version",
            "engine_build_sha256",
            "prorated_rules_applied",
            "employee",
            "employer",
            "annual_projection",
            "breakdown",
            "citations",
            "warnings",
        ]
        .map(|k| a.find(&format!("\"{k}\"")).expect(k));
        for w in positions.windows(2) {
            assert!(w[0] < w[1], "top-level keys must serialize in stable order");
        }
    }

    // Test 31: breakdown contains every §9.3 factor; all present even when zero —
    // never omitted, never null. A missing key reads as "we didn't compute it."
    #[test]
    fn breakdown_includes_every_factor_even_when_zero() {
        let encoded = serde_json::to_value(sample_response()).unwrap();
        let b = encoded.get("breakdown").unwrap().as_object().unwrap();
        for key in Breakdown::FACTOR_KEYS {
            let v = b
                .get(*key)
                .unwrap_or_else(|| panic!("missing breakdown factor `{key}`"));
            assert!(!v.is_null(), "breakdown.{key} must not be null");
        }
        assert_eq!(b.len(), Breakdown::FACTOR_KEYS.len());
        assert!(
            b.get("a").is_none(),
            "JSON keys are T4127 symbols; lowercase 'a' is not factor A"
        );
        let json = serde_json::to_string(&sample_response().breakdown).unwrap();
        let mut last = 0;
        for key in Breakdown::FACTOR_KEYS {
            let needle = format!("\"{key}\"");
            let pos = json
                .find(&needle)
                .unwrap_or_else(|| panic!("missing {key}"));
            assert!(
                pos >= last,
                "{key} must serialize after the previous T4127 factor"
            );
            last = pos;
        }
    }

    /// Every breakdown key is documented in `data/factors.json` (website /docs/factors
    /// and calculator tooltips). Symbol → T4127 chapter reference → one-line definition.
    fn factors_catalog_matches(raw: &str) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Catalog {
            factors: Vec<FactorDoc>,
        }
        #[derive(Deserialize)]
        struct FactorDoc {
            symbol: String,
            t4127_reference: String,
            definition: String,
        }
        let catalog: Catalog =
            serde_json::from_str(raw).map_err(|e| format!("data/factors.json parses: {e}"))?;
        let mut by_symbol = std::collections::BTreeMap::new();
        for row in &catalog.factors {
            if row.t4127_reference.is_empty() || row.definition.is_empty() {
                return Err(format!("{} missing reference or definition", row.symbol));
            }
            if by_symbol.insert(row.symbol.as_str(), row).is_some() {
                return Err(format!("duplicate symbol {}", row.symbol));
            }
        }
        for key in Breakdown::FACTOR_KEYS {
            if !by_symbol.contains_key(key) {
                return Err(format!(
                    "breakdown key `{key}` missing from data/factors.json"
                ));
            }
        }
        for symbol in by_symbol.keys() {
            if !Breakdown::FACTOR_KEYS.contains(symbol) {
                return Err(format!(
                    "data/factors.json has `{symbol}` which is not a breakdown key"
                ));
            }
        }
        if catalog.factors.len() != Breakdown::FACTOR_KEYS.len() {
            return Err(format!(
                "catalog len {} != FACTOR_KEYS len {}",
                catalog.factors.len(),
                Breakdown::FACTOR_KEYS.len()
            ));
        }
        Ok(())
    }

    #[test]
    fn every_breakdown_key_is_in_factors_json() {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/factors.json"
        ));
        factors_catalog_matches(raw).expect("factors.json matches FACTOR_KEYS");
    }

    /// A passing consistency check is not evidence that the check measures
    /// anything. Break the catalog on purpose; the helper must fail.
    #[test]
    fn factors_json_consistency_fails_when_broken_on_purpose() {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/factors.json"
        ));
        factors_catalog_matches(raw).expect("committed catalog is in sync");

        let mut extra_val: serde_json::Value = serde_json::from_str(raw).unwrap();
        extra_val["factors"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "symbol": "ZZZ_NOT_A_FACTOR",
                "t4127_reference": "none",
                "definition": "mutation"
            }));
        let extra = serde_json::to_string(&extra_val).unwrap();
        let extra_err =
            factors_catalog_matches(&extra).expect_err("extra catalog symbol must fail");
        assert!(extra_err.contains("ZZZ_NOT_A_FACTOR"), "{extra_err}");

        let mut dropped_val: serde_json::Value = serde_json::from_str(raw).unwrap();
        dropped_val["factors"].as_array_mut().unwrap().pop();
        let dropped = serde_json::to_string(&dropped_val).unwrap();
        let dropped_err = factors_catalog_matches(&dropped).expect_err("removed symbol must fail");
        assert!(dropped_err.contains("QPIP"), "{dropped_err}");
    }

    /// 35. engine_version from CARGO_PKG_VERSION; build sha is a 64-hex source digest.
    #[test]
    fn engine_version_and_build_sha_placeholder() {
        let r = sample_response();
        assert_eq!(r.engine_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(r.engine_build_sha256.len(), 64);
        assert!(
            r.engine_build_sha256.chars().all(|c| c.is_ascii_hexdigit()),
            "engine_build_sha256 must be hex, got {}",
            r.engine_build_sha256
        );
        assert_eq!(ENGINE_BUILD_SHA256, r.engine_build_sha256.as_str());
        let _ = zero_rate();
    }

    /// 35b. engine_build_sha256 must be path-independent: hashing `src/**/*.rs` with
    /// paths relative to `src/` (not `CARGO_MANIFEST_DIR` absolute paths). Absolute
    /// paths made WSL vs GitHub Actions produce different WASM hashes for the same
    /// tree.
    #[test]
    fn engine_build_sha_is_independent_of_checkout_directory() {
        use std::path::{Path, PathBuf};
        use std::process::Command;

        fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    collect_rs(&path, out);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
        }

        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        collect_rs(&src, &mut files);
        files.sort();

        let mut listing = String::new();
        for f in &files {
            let out = Command::new("sha256sum").arg(f).output().unwrap();
            assert!(out.status.success(), "sha256sum {}", f.display());
            let hex = String::from_utf8_lossy(&out.stdout)
                .split_whitespace()
                .next()
                .unwrap()
                .to_string();
            let rel = f.strip_prefix(&src).unwrap();
            let rel = rel.to_string_lossy().replace('\\', "/");
            listing.push_str(&hex);
            listing.push_str("  ");
            listing.push_str(&rel);
            listing.push('\n');
        }
        let out = Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(listing.as_bytes())?;
                child.wait_with_output()
            })
            .unwrap();
        assert!(out.status.success());
        let want = String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        assert_eq!(
            ENGINE_BUILD_SHA256, want,
            "build.rs must hash relative paths under src/; absolute paths break CI"
        );
    }
}

/// Top-level §9.3 response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub rule_set_version: String,
    pub engine_version: String,
    pub engine_build_sha256: String,
    pub prorated_rules_applied: bool,
    pub employee: EmployeeAmounts,
    pub employer: EmployerAmounts,
    pub annual_projection: AnnualProjection,
    pub breakdown: Breakdown,
    pub citations: Vec<Citation>,
    pub warnings: Vec<Warning>,
}

impl Response {
    /// Engine version string baked at compile time.
    pub fn engine_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Build-id placeholder until step 12 wires CI.
    pub fn engine_build_sha256() -> &'static str {
        ENGINE_BUILD_SHA256
    }
}

/// Per-period employee withholdings and net pay.
///
/// `federal_tax` is T4127 `[(T1)/P]+L` — additional tax L is not a sibling
/// field. `total_tax` is `federal_tax + provincial_tax` (the sum of the two
/// PDOC-displayed, separately rounded lines). That can differ by one cent
/// from `breakdown.T`, which is T4127 Step 6 `round((T1+T2)/P)+L`. Prefer
/// `total_tax` when comparing to a screen total a user can add up.
/// `total_deductions` is tax + CPP + CPP2 + EI + QPIP + union dues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeAmounts {
    pub federal_tax: Money,
    pub provincial_tax: Money,
    pub total_tax: Money,
    pub cpp: Money,
    pub cpp2: Money,
    pub ei: Money,
    pub qpip: Money,
    pub total_deductions: Money,
    pub net_pay: Money,
}

/// Per-period employer contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployerAmounts {
    pub cpp: Money,
    pub cpp2: Money,
    pub ei: Money,
    pub qpip: Money,
}

/// Annualised projection (Option 1 annual figures / Option 2 projected year).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnualProjection {
    pub taxable_income: Money,
    pub federal_tax: Money,
    pub provincial_tax: Money,
    pub cpp: Money,
    pub cpp2: Money,
    pub ei: Money,
    pub qpip: Money,
}

/// Every T4127 factor listed for §9.3 breakdown. Always populated; zero means
/// computed-as-zero, not omitted. JSON keys are the T4127 symbols (ADR: domain
/// legibility over snake_case).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Breakdown {
    #[serde(rename = "A")]
    pub a: Money,
    #[serde(rename = "R")]
    pub r: Rate,
    #[serde(rename = "K")]
    pub k: Money,
    #[serde(rename = "K1")]
    pub k1: Money,
    #[serde(rename = "K2")]
    pub k2: Money,
    #[serde(rename = "K4")]
    pub k4: Money,
    #[serde(rename = "T3")]
    pub t3: Money,
    #[serde(rename = "T1")]
    pub t1: Money,
    #[serde(rename = "V")]
    pub v: Rate,
    #[serde(rename = "KP")]
    pub kp: Money,
    #[serde(rename = "K1P")]
    pub k1p: Money,
    #[serde(rename = "K2P")]
    pub k2p: Money,
    #[serde(rename = "T4")]
    pub t4: Money,
    #[serde(rename = "V1")]
    pub v1: Money,
    #[serde(rename = "V2")]
    pub v2: Money,
    #[serde(rename = "S")]
    pub s: Money,
    #[serde(rename = "T2")]
    pub t2: Money,
    #[serde(rename = "T")]
    pub t: Money,
    #[serde(rename = "TB")]
    pub tb: Money,
    #[serde(rename = "M")]
    pub m: Money,
    #[serde(rename = "M1")]
    pub m1: Money,
    #[serde(rename = "C")]
    pub c: Money,
    #[serde(rename = "C2")]
    pub c2: Money,
    #[serde(rename = "EI")]
    pub ei: Money,
    #[serde(rename = "F5")]
    pub f5: Money,
    #[serde(rename = "BPAF")]
    pub bpaf: Money,
    #[serde(rename = "K3")]
    pub k3: Money,
    #[serde(rename = "K3P")]
    pub k3p: Money,
    #[serde(rename = "K4P")]
    pub k4p: Money,
    #[serde(rename = "K5P")]
    pub k5p: Money,
    #[serde(rename = "LCF")]
    pub lcf: Money,
    #[serde(rename = "LCP")]
    pub lcp: Money,
    #[serde(rename = "Y")]
    pub y: Money,
    #[serde(rename = "F5A")]
    pub f5a: Money,
    #[serde(rename = "F5B")]
    pub f5b: Money,
    #[serde(rename = "D")]
    pub d: Money,
    #[serde(rename = "D1")]
    pub d1: Money,
    #[serde(rename = "D2")]
    pub d2: Money,
    #[serde(rename = "P")]
    pub p: Count,
    #[serde(rename = "PR")]
    pub pr: Count,
    #[serde(rename = "PM")]
    pub pm: Count,
    #[serde(rename = "S1")]
    pub s1: S1,
    #[serde(rename = "CEA")]
    pub cea: Money,
    #[serde(rename = "TC")]
    pub tc: Money,
    #[serde(rename = "TCP")]
    pub tcp: Money,
    #[serde(rename = "IE")]
    pub ie: Money,
    #[serde(rename = "QPIP")]
    pub qpip: Money,
}

/// Integer T4127 factor (P, PR, PM) serialised as a lexical string, never a JSON number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Count(pub u16);

impl Count {
    /// Wrap a period or month count.
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

impl Serialize for Count {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<u16>()
            .map(Count)
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Breakdown {
    /// Canonical factor key set (spec §9.3). Order matches struct fields.
    pub const FACTOR_KEYS: &'static [&'static str] = &[
        "A", "R", "K", "K1", "K2", "K4", "T3", "T1", "V", "KP", "K1P", "K2P", "T4", "V1", "V2",
        "S", "T2", "T", "TB", "M", "M1", "C", "C2", "EI", "F5", "BPAF", "K3", "K3P", "K4P", "K5P",
        "LCF", "LCP", "Y", "F5A", "F5B", "D", "D1", "D2", "P", "PR", "PM", "S1", "CEA", "TC",
        "TCP", "IE", "QPIP",
    ];

    /// All factors zero — the honest "not yet computed" filled shape for tests.
    pub fn zeros() -> Self {
        let z = Money::ZERO;
        let zr = Rate::parse("0").expect("0 rate");
        let zc = Count::new(0);
        Self {
            a: z,
            r: zr,
            k: z,
            k1: z,
            k2: z,
            k4: z,
            t3: z,
            t1: z,
            v: zr,
            kp: z,
            k1p: z,
            k2p: z,
            t4: z,
            v1: z,
            v2: z,
            s: z,
            t2: z,
            t: z,
            tb: z,
            m: z,
            m1: z,
            c: z,
            c2: z,
            ei: z,
            f5: z,
            bpaf: z,
            k3: z,
            k3p: z,
            k4p: z,
            k5p: z,
            lcf: z,
            lcp: z,
            y: z,
            f5a: z,
            f5b: z,
            d: z,
            d1: z,
            d2: z,
            p: zc,
            pr: zc,
            pm: zc,
            s1: S1::zero_placeholder(),
            cea: z,
            tc: z,
            tcp: z,
            ie: z,
            qpip: z,
        }
    }
}

/// Provenance pointer for a computed factor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub factor: String,
    pub source_document: String,
    pub source_url: String,
}

/// Non-fatal note for the caller (e.g. claim-code E with OHP still due).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Warning {
    pub code: String,
    pub message: String,
}

/// Optional bag for extension metadata without breaking deny_unknown on Response.
/// Not part of the §9.3 wire shape; kept for internal maps that need stable order.
#[allow(dead_code)]
type StableMap<T> = BTreeMap<String, T>;
