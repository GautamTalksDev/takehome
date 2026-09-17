//! Emit rule JSON in the schema takehome-core already loads.

use crate::archive::ArchiveMeta;
use crate::error::IngestError;
use crate::provinces::TABLE_8_1;
use crate::tables::{BasicCell, Bracket, ClaimTable, OtherAmounts, ParsedBundle};
use crate::tokens::SpecialToken;
use crate::{ingest_git_sha, tool_version};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Overlay a delta edition onto a previously ingested complete rule directory.
#[derive(Debug, Clone, Copy)]
pub struct Overlay<'a> {
    pub base_rules_dir: Option<&'a Path>,
    pub enabled: bool,
}

pub fn write_rule_directory(
    out_dir: &Path,
    meta: &ArchiveMeta,
    bundle: &ParsedBundle,
    overlay: Overlay<'_>,
) -> Result<Vec<String>, IngestError> {
    fs::create_dir_all(out_dir)
        .map_err(|e| IngestError::archive(out_dir, format!("mkdir failed: {e}")))?;

    let mut origin: BTreeMap<String, String> = BTreeMap::new();
    let mut written = Vec::new();
    let delta_codes: Vec<String> = if meta.is_delta() {
        meta.delta_jurisdictions.clone()
    } else {
        bundle.brackets.keys().cloned().collect()
    };

    for (code, brackets) in &bundle.brackets {
        if meta.is_delta() && !delta_codes.iter().any(|d| d == code) {
            continue;
        }
        let filename = jurisdiction_filename(code);
        let json = jurisdiction_json(code, brackets, bundle)?;
        write_json(&out_dir.join(&filename), &json)?;
        written.push(filename);
        origin.insert(
            code.clone(),
            if meta.is_delta() { "delta" } else { "complete" }.to_string(),
        );
    }

    if meta.is_delta() {
        if overlay.enabled {
            let base = overlay.base_rules_dir.ok_or_else(|| {
                IngestError::message(
                    "delta edition requires --base <january-rules-dir> to overlay unchanged jurisdictions",
                )
            })?;
            copy_inherited(base, out_dir, &delta_codes, &mut origin, &mut written)?;
        }
        let missing = missing_table_8_1(&origin);
        if !missing.is_empty() {
            return Err(IngestError::message(format!(
                "delta edition is incomplete without overlay; missing jurisdictions: {}",
                missing.join(", ")
            )));
        }
    }

    if bundle.cpp.is_object() {
        write_json(&out_dir.join("cpp.json"), &bundle.cpp)?;
        written.push("cpp.json".to_string());
    }
    if bundle.ei.is_object() {
        write_json(&out_dir.join("ei.json"), &bundle.ei)?;
        written.push("ei.json".to_string());
    }
    if bundle.qpip.is_object() {
        write_json(&out_dir.join("qpip.json"), &bundle.qpip)?;
        written.push("qpip.json".to_string());
    }

    write_claim_codes(out_dir, bundle, overlay, &mut written)?;

    written.sort();
    written.dedup();
    let manifest = manifest_json(meta, &written, &origin)?;
    write_json(&out_dir.join("manifest.json"), &manifest)?;
    written.push("manifest.json".to_string());
    Ok(written)
}

fn copy_inherited(
    base: &Path,
    out_dir: &Path,
    delta_codes: &[String],
    origin: &mut BTreeMap<String, String>,
    written: &mut Vec<String>,
) -> Result<(), IngestError> {
    for code in TABLE_8_1 {
        if delta_codes.iter().any(|d| d == code) {
            continue;
        }
        let filename = jurisdiction_filename(code);
        let src = base.join(&filename);
        let bytes = fs::read(&src).map_err(|e| {
            IngestError::archive(&src, format!("overlay base missing {filename}: {e}"))
        })?;
        fs::write(out_dir.join(&filename), bytes)
            .map_err(|e| IngestError::archive(out_dir, format!("write {filename}: {e}")))?;
        written.push(filename);
        origin.insert((*code).to_string(), "inherited".to_string());
    }
    for name in ["cpp.json", "ei.json", "qpip.json"] {
        let src = base.join(name);
        if src.exists() && !out_dir.join(name).exists() {
            let bytes = fs::read(&src)
                .map_err(|e| IngestError::archive(&src, format!("read {name}: {e}")))?;
            fs::write(out_dir.join(name), bytes)
                .map_err(|e| IngestError::archive(out_dir, format!("write {name}: {e}")))?;
            written.push(name.to_string());
        }
    }
    Ok(())
}

fn missing_table_8_1(origin: &BTreeMap<String, String>) -> Vec<String> {
    TABLE_8_1
        .iter()
        .filter(|code| !origin.contains_key(**code))
        .map(|c| (*c).to_string())
        .collect()
}

fn jurisdiction_filename(code: &str) -> String {
    match code {
        "FED" => "federal.json".to_string(),
        "OUTSIDE_CANADA" => "outside_canada.json".to_string(),
        other => format!("{}.json", other.to_ascii_lowercase()),
    }
}

fn write_claim_codes(
    out_dir: &Path,
    bundle: &ParsedBundle,
    overlay: Overlay<'_>,
    written: &mut Vec<String>,
) -> Result<(), IngestError> {
    if bundle.claim_tables.is_empty() && overlay.base_rules_dir.is_none() {
        return Ok(());
    }
    let dir = out_dir.join("claim-codes");
    fs::create_dir_all(&dir).map_err(|e| IngestError::archive(&dir, format!("mkdir: {e}")))?;

    if let Some(base) = overlay.base_rules_dir {
        let base_cc = base.join("claim-codes");
        if base_cc.is_dir() {
            let entries = fs::read_dir(&base_cc)
                .map_err(|e| IngestError::archive(&base_cc, format!("read: {e}")))?;
            for entry in entries {
                let entry =
                    entry.map_err(|e| IngestError::archive(&base_cc, format!("entry: {e}")))?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let stem = name_str.trim_end_matches(".json").to_ascii_uppercase();
                let stem = if stem == "FEDERAL" {
                    "FED".to_string()
                } else {
                    stem
                };
                if bundle.claim_tables.contains_key(&stem) {
                    continue;
                }
                let bytes = fs::read(entry.path())
                    .map_err(|e| IngestError::archive(&entry.path(), format!("read: {e}")))?;
                let dest = dir.join(&name);
                fs::write(&dest, bytes)
                    .map_err(|e| IngestError::archive(&dest, format!("write: {e}")))?;
                written.push(format!("claim-codes/{name_str}"));
            }
        }
    }

    for (code, table) in &bundle.claim_tables {
        let filename = format!("{}.json", code.to_ascii_lowercase());
        write_json(&dir.join(&filename), &claim_table_json(code, table))?;
        written.push(format!("claim-codes/{filename}"));
    }
    Ok(())
}

fn claim_table_json(code: &str, table: &ClaimTable) -> Value {
    let rows: Vec<Value> = table
        .rows
        .iter()
        .map(|r| {
            json!({
                "code": r.code,
                "claim_from": r.claim_from,
                "claim_to": r.claim_to,
                "tcp": r.tcp,
                "k1p": r.k1p
            })
        })
        .collect();
    json!({
        "jurisdiction": code,
        "title": table.title,
        "rows": rows
    })
}

fn write_json(path: &Path, value: &Value) -> Result<(), IngestError> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|e| IngestError::archive(path, format!("json: {e}")))?;
    text.push('\n');
    fs::write(path, text).map_err(|e| IngestError::archive(path, format!("write failed: {e}")))
}

pub fn jurisdiction_json(
    code: &str,
    brackets: &[Bracket],
    bundle: &ParsedBundle,
) -> Result<Value, IngestError> {
    let other = bundle.other.get(code).cloned().unwrap_or_default();
    let option1_brackets = option1_brackets(code, brackets, bundle);
    let mut map = Map::new();
    map.insert("brackets".into(), brackets_value(&option1_brackets));
    map.insert(
        "lowest_rate".into(),
        Value::String(option1_brackets[0].rate.clone()),
    );
    map.insert(
        "basic_personal_amount".into(),
        basic_personal_amount(code, brackets, &other, bundle)?,
    );
    if let Some(cea) = &other.cea {
        map.insert(
            "canada_employment_amount".into(),
            Value::String(cea.clone()),
        );
    }
    if let Some(idx) = &other.index_rate {
        map.insert("index_rate".into(), Value::String(idx.clone()));
    }
    if let (Some(rate), Some(max)) = (&other.lcp_rate, &other.lcp_amount) {
        let key = if code == "FED" { "lcf" } else { "lcp" };
        map.insert(
            key.into(),
            json!({
                "rate": rate,
                "max": max
            }),
        );
    }
    insert_tax_reduction(&mut map, code, &other, bundle)?;
    if !other.surtax.is_empty() {
        let tiers: Vec<Value> = other
            .surtax
            .iter()
            .map(|(t, r)| json!({"threshold": t, "rate": r}))
            .collect();
        map.insert("surtax".into(), Value::Array(tiers));
    }
    if let Some(hp) = supplement_array(bundle, code, "health_premium") {
        map.insert("health_premium".into(), hp);
    }
    if let Some(ab) = &other.abatement {
        map.insert("abatement".into(), Value::String(ab.clone()));
    }
    if let Some(s) = &other.surtax_flat {
        map.insert("surtax_flat".into(), Value::String(s.clone()));
    }
    if code == "FED" {
        if !map.contains_key("abatement") {
            if let Some(ab) = bundle.other.get("QC").and_then(|o| o.abatement.clone()) {
                map.insert("abatement".into(), Value::String(ab));
            }
        }
        if !map.contains_key("surtax_flat") {
            if let Some(s) = bundle
                .other
                .get("OUTSIDE_CANADA")
                .and_then(|o| o.surtax_flat.clone())
            {
                map.insert("surtax_flat".into(), Value::String(s));
            }
        }
    }
    if bundle
        .supplement
        .get(code)
        .and_then(|v| v.get("supplemental_credit"))
        .is_some()
    {
        map.insert("supplemental_credit".into(), json!({}));
    }
    apply_option2_scope(code, map, bundle)
}

fn option1_brackets(code: &str, brackets: &[Bracket], bundle: &ParsedBundle) -> Vec<Bracket> {
    let mut out = brackets.to_vec();
    let prorate_index = match code {
        "BC" if bundle
            .supplement
            .get("BC")
            .and_then(|v| v.get("option2_brackets"))
            .is_some() =>
        {
            Some(0)
        }
        "PE" if bundle
            .supplement
            .get("PE")
            .and_then(|v| v.get("option2_sixth_bracket"))
            .is_some()
            && out.len() >= 6 =>
        {
            Some(out.len() - 1)
        }
        _ => None,
    };
    if let Some(i) = prorate_index {
        if let Some(b) = out.get_mut(i) {
            b.prorated = true;
        }
    }
    out
}

fn insert_tax_reduction(
    map: &mut Map<String, Value>,
    code: &str,
    other: &OtherAmounts,
    bundle: &ParsedBundle,
) -> Result<(), IngestError> {
    let Some(basic) = &other.s2 else {
        return Ok(());
    };
    // Factor Y (dependant) is not in the CSV bundle. Emit tax_reduction only
    // when supplement.json supplies it — never invent a number.
    // BC stores the S phase-out upper threshold ($44,952) in `dependant`
    // because TaxReduction has no other slot (schema frozen).
    if let Some(dependant) = optional_supplement_str(bundle, code, "tax_reduction_dependant") {
        map.insert(
            "tax_reduction".into(),
            json!({
                "basic": basic,
                "dependant": dependant
            }),
        );
    }
    Ok(())
}

fn apply_option2_scope(
    code: &str,
    mut map: Map<String, Value>,
    bundle: &ParsedBundle,
) -> Result<Value, IngestError> {
    match code {
        "BC" if bundle
            .supplement
            .get("BC")
            .and_then(|v| v.get("option2_brackets"))
            .is_some() =>
        {
            let option2_brackets =
                supplement_array(bundle, code, "option2_brackets").ok_or_else(|| {
                    IngestError::message("supplement.json missing BC.option2_brackets")
                })?;
            let option2_s2 = supplement_str(bundle, code, "option2_s2")?;
            let option1_brackets = map.remove("brackets").expect("brackets");
            let option1_lowest = map.remove("lowest_rate").expect("lowest_rate");
            let option1_reduction = map.remove("tax_reduction").ok_or_else(|| {
                IngestError::message("BC option1 tax_reduction missing (S2 + 44952)")
            })?;
            let option2_lowest = option2_brackets
                .as_array()
                .and_then(|a| a.first())
                .and_then(|b| b.get("rate"))
                .cloned()
                .ok_or_else(|| IngestError::message("BC option2 first rate missing"))?;
            let option2_reduction = json!({
                "basic": option2_s2,
                "dependant": option1_reduction.get("dependant").cloned().unwrap_or(Value::Null)
            });
            map.insert(
                "brackets".into(),
                per_option(option1_brackets, option2_brackets),
            );
            map.insert(
                "lowest_rate".into(),
                per_option(option1_lowest, option2_lowest),
            );
            map.insert(
                "tax_reduction".into(),
                per_option(option1_reduction, option2_reduction),
            );
        }
        "NL" => {
            if let Some(option2_bpa) =
                optional_supplement_str(bundle, code, "option2_basic_personal_amount")
            {
                let option1_bpa = map
                    .remove("basic_personal_amount")
                    .expect("basic_personal_amount");
                map.insert(
                    "basic_personal_amount".into(),
                    per_option(option1_bpa, json!({"type": "fixed", "amount": option2_bpa})),
                );
            }
        }
        "PE" => {
            if let Some(sixth) = bundle
                .supplement
                .get(code)
                .and_then(|v| v.get("option2_sixth_bracket"))
                .cloned()
            {
                let option1_brackets = map.remove("brackets").expect("brackets");
                let mut option2 = option1_brackets.clone();
                if let Some(arr) = option2.as_array_mut() {
                    for b in arr.iter_mut() {
                        if let Some(obj) = b.as_object_mut() {
                            obj.remove("prorated");
                        }
                    }
                    if arr.len() >= 6 {
                        arr[5] = sixth;
                    } else {
                        arr.push(sixth);
                    }
                }
                map.insert("brackets".into(), per_option(option1_brackets, option2));
            }
        }
        _ => {}
    }
    Ok(Value::Object(map))
}

fn per_option(option1: Value, option2: Value) -> Value {
    json!({
        "option1": option1,
        "option2": option2
    })
}

fn brackets_value(brackets: &[Bracket]) -> Value {
    Value::Array(
        brackets
            .iter()
            .map(|b| {
                let mut obj = Map::new();
                obj.insert("threshold".into(), Value::String(b.threshold.clone()));
                obj.insert("rate".into(), Value::String(b.rate.clone()));
                obj.insert("constant".into(), Value::String(b.constant.clone()));
                if b.prorated {
                    obj.insert("prorated".into(), Value::Bool(true));
                }
                Value::Object(obj)
            })
            .collect(),
    )
}

fn basic_personal_amount(
    code: &str,
    brackets: &[Bracket],
    other: &OtherAmounts,
    bundle: &ParsedBundle,
) -> Result<Value, IngestError> {
    match other.basic.as_ref() {
        Some(BasicCell::Token(SpecialToken::Bpaf)) => {
            let max = bundle
                .claim_code_1_tc
                .get(code)
                .ok_or_else(|| {
                    IngestError::message(format!("{code}: BPAF max from claim code 1 missing"))
                })?
                .clone();
            let min = supplement_str(bundle, code, "bpa_min")?;
            let phaseout_start = brackets
                .get(3)
                .ok_or_else(|| IngestError::message(format!("{code}: BPAF needs 4th threshold")))?
                .threshold
                .clone();
            let phaseout_end = brackets
                .get(4)
                .ok_or_else(|| IngestError::message(format!("{code}: BPAF needs 5th threshold")))?
                .threshold
                .clone();
            Ok(json!({
                "type": "dynamic",
                "formula": {},
                "max": max,
                "min": min,
                "phaseout_start": phaseout_start,
                "phaseout_end": phaseout_end
            }))
        }
        Some(BasicCell::Token(SpecialToken::Bpamb)) => {
            let max = bundle
                .claim_code_1_tc
                .get(code)
                .ok_or_else(|| {
                    IngestError::message(format!("{code}: BPAMB max from claim code 1 missing"))
                })?
                .clone();
            Ok(json!({
                "type": "dynamic",
                "formula": {},
                "max": max,
                "min": "0.00",
                "phaseout_start": "200000.00",
                "phaseout_end": "400000.00"
            }))
        }
        Some(BasicCell::Token(SpecialToken::Bpayt)) => Ok(json!({"type": "same_as_federal"})),
        Some(BasicCell::Token(SpecialToken::NoClaimAmount)) => {
            Ok(json!({"type": "not_applicable"}))
        }
        Some(BasicCell::Amount(amount)) => Ok(json!({"type": "fixed", "amount": amount})),
        None => Ok(json!({"type": "not_applicable"})),
    }
}

fn supplement_str(bundle: &ParsedBundle, code: &str, key: &str) -> Result<String, IngestError> {
    bundle
        .supplement
        .get(code)
        .and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            IngestError::message(format!(
                "supplement.json missing {code}.{key} (not published in the CSV bundle)"
            ))
        })
}

fn optional_supplement_str(bundle: &ParsedBundle, code: &str, key: &str) -> Option<String> {
    bundle
        .supplement
        .get(code)
        .and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn supplement_array(bundle: &ParsedBundle, code: &str, key: &str) -> Option<Value> {
    bundle
        .supplement
        .get(code)
        .and_then(|v| v.get(key))
        .cloned()
}

fn manifest_json(
    meta: &ArchiveMeta,
    written: &[String],
    origin: &BTreeMap<String, String>,
) -> Result<Value, IngestError> {
    let archive_files: Vec<Value> = meta
        .files
        .iter()
        .map(|f| json!({"name": f.name, "sha256": f.sha256}))
        .collect();
    let primary = meta
        .files
        .iter()
        .find(|f| {
            f.name.starts_with("rtsncmtrshldcnstnt-")
                || f.name.contains("rates-income-thresholds-constants")
        })
        .map(|f| f.sha256.as_str())
        .unwrap_or("");
    let mut files: Vec<String> = written
        .iter()
        .filter(|f| f.ends_with(".json") && *f != "manifest.json" && !f.starts_with("claim-codes/"))
        .cloned()
        .collect();
    files.sort();
    files.dedup();
    let mut manifest = json!({
        "rule_set_version": meta.effective_from,
        "effective_from": meta.effective_from,
        "effective_to": null,
        "published_by": meta.published_by,
        "sources": [{
            "source_document": meta.source_document,
            "source_url": meta.source_url,
            "retrieved_at": meta.retrieved_at,
            "source_sha256": primary,
            "files": files,
            "tool_version": tool_version(),
            "ingest_git_sha": ingest_git_sha(),
            "archive_files": archive_files
        }]
    });
    if meta.is_delta() {
        manifest["edition_kind"] = json!("delta");
        manifest["delta_marker"] = json!(meta.delta_marker.clone());
        manifest["jurisdiction_origin"] = json!(origin);
    }
    Ok(manifest)
}
