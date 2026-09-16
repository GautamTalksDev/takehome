//! Table parsers. Unknown row type / column / count → hard error.

use crate::archive::{read_verified, ArchiveMeta, EditionKind};
use crate::encoding::decode_bytes;
use crate::error::IngestError;
use crate::lexical::{as_money, as_rate};
use crate::provinces::{cra_code_to_jurisdiction, filename_to_jurisdiction};
use crate::tokens::{special_token, SpecialToken};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bracket {
    pub threshold: String,
    pub rate: String,
    pub constant: String,
    pub prorated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct OtherAmounts {
    pub basic: Option<BasicCell>,
    pub index_rate: Option<String>,
    pub lcp_rate: Option<String>,
    pub lcp_amount: Option<String>,
    pub cea: Option<String>,
    pub s2: Option<String>,
    pub surtax: Vec<(String, String)>,
    pub abatement: Option<String>,
    pub surtax_flat: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasicCell {
    Token(SpecialToken),
    Amount(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimCodeRow {
    pub code: String,
    pub claim_from: String,
    pub claim_to: String,
    pub tcp: String,
    pub k1p: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimTable {
    pub title: String,
    pub rows: Vec<ClaimCodeRow>,
}

#[derive(Debug, Clone)]
pub struct ParsedBundle {
    pub brackets: BTreeMap<String, Vec<Bracket>>,
    pub other: BTreeMap<String, OtherAmounts>,
    pub claim_code_1_tc: BTreeMap<String, String>,
    pub claim_tables: BTreeMap<String, ClaimTable>,
    pub cpp: Value,
    pub ei: Value,
    pub qpip: Value,
    pub supplement: Value,
}

impl ParsedBundle {
    pub fn from_archive(dir: &Path, meta: &ArchiveMeta) -> Result<Self, IngestError> {
        let names: Vec<&str> = meta.files.iter().map(|f| f.name.as_str()).collect();
        let brackets = parse_named(dir, &names, table_8_1_name, parse_bracket_table)?;
        let other = parse_named(dir, &names, table_8_2_name, parse_other_amounts)?;
        let optional_cpp_ei = meta.edition_kind == EditionKind::Delta;
        let cpp = match parse_cpp(dir, &names) {
            Ok(v) => v,
            Err(_) if optional_cpp_ei => Value::Null,
            Err(e) => return Err(e),
        };
        let ei = match parse_ei(dir, &names) {
            Ok(v) => v,
            Err(_) if optional_cpp_ei => Value::Null,
            Err(e) => return Err(e),
        };
        let qpip = match parse_qpip(dir, &names) {
            Ok(v) => v,
            Err(_) if optional_cpp_ei => Value::Null,
            Err(e) => return Err(e),
        };
        let mut claim_code_1_tc = BTreeMap::new();
        let mut claim_tables = BTreeMap::new();
        for name in &names {
            if name.starts_with("cc-") {
                let (code, table) = parse_claim_table(dir, name)?;
                if let Some(row) = table.rows.iter().find(|r| r.code == "1") {
                    claim_code_1_tc.insert(code.clone(), row.tcp.clone());
                } else {
                    return Err(IngestError::message(format!(
                        "{name}: missing claim code 1 row"
                    )));
                }
                claim_tables.insert(code, table);
            }
        }
        let supplement = load_supplement(dir)?;
        Ok(Self {
            brackets,
            other,
            claim_code_1_tc,
            claim_tables,
            cpp,
            ei,
            qpip,
            supplement,
        })
    }
}

fn parse_named<T>(
    dir: &Path,
    names: &[&str],
    find: for<'a> fn(&'a [&'a str]) -> Result<&'a str, IngestError>,
    parse: fn(&str, &str) -> Result<T, IngestError>,
) -> Result<T, IngestError> {
    let name = find(names)?;
    let (_path, bytes) = read_verified(dir, name)?;
    let decoded = decode_bytes(&bytes);
    parse(name, &decoded.text)
}

fn table_8_1_name<'a>(names: &'a [&str]) -> Result<&'a str, IngestError> {
    names
        .iter()
        .copied()
        .find(|n| {
            n.starts_with("rtsncmtrshldcnstnt-") || n.contains("rates-income-thresholds-constants")
        })
        .ok_or_else(|| IngestError::message("archive missing Table 8.1 csv"))
}

fn table_8_2_name<'a>(names: &'a [&str]) -> Result<&'a str, IngestError> {
    names
        .iter()
        .copied()
        .find(|n| n.starts_with("thrrtsmnts-") || n.contains("other-rates-amounts"))
        .ok_or_else(|| IngestError::message("archive missing Table 8.2 csv"))
}

fn load_supplement(dir: &Path) -> Result<Value, IngestError> {
    let path = dir.join("supplement.json");
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| IngestError::archive(&path, format!("read failed: {e}")))?;
    serde_json::from_str(&raw)
        .map_err(|e| IngestError::archive(&path, format!("invalid supplement.json: {e}")))
}

pub fn csv_rows(file: &str, text: &str) -> Result<Vec<(usize, Vec<String>, String)>, IngestError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for rec in reader.records() {
        let rec = rec.map_err(|e| IngestError::message(format!("{file}: csv: {e}")))?;
        let line = match rec.position() {
            Some(p) => usize::try_from(p.line())
                .map_err(|_| IngestError::message(format!("{file}: line number overflow")))?,
            None => 0,
        };
        let fields: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
        let content = fields.join(",");
        rows.push((line, fields, content));
    }
    Ok(rows)
}

fn cell(fields: &[String], i: usize) -> &str {
    fields.get(i).map(String::as_str).unwrap_or("")
}

/// Empty, or a CRA dash standing in for a missing numeric / amount.
fn is_absent(v: &str) -> bool {
    let v = v.trim();
    v.is_empty() || matches!(v, "–" | "-" | "—" | "−")
}

fn is_empty_row(fields: &[String]) -> bool {
    fields.iter().all(|c| c.trim().is_empty())
}

/// Table 8.1: three lines per jurisdiction (A, R/V, K/KP).
pub fn parse_bracket_table(
    file: &str,
    text: &str,
) -> Result<BTreeMap<String, Vec<Bracket>>, IngestError> {
    let rows = csv_rows(file, text)?;
    let mut out: BTreeMap<String, Vec<Bracket>> = BTreeMap::new();
    let mut i = 0;
    let mut saw_header = false;
    while i < rows.len() {
        let (line, fields, content) = &rows[i];
        if is_empty_row(fields) {
            i += 1;
            continue;
        }
        let c0 = cell(fields, 0).trim();
        let c1 = cell(fields, 1).trim();
        if c0.starts_with("Table") {
            i += 1;
            continue;
        }
        if c0.starts_with('*') {
            i += 1;
            continue;
        }
        if c0.is_empty() && c1.is_empty() && cell(fields, 2).trim() == "1st" {
            saw_header = true;
            i += 1;
            continue;
        }
        if !saw_header {
            return Err(IngestError::row(
                file,
                *line,
                content,
                "unknown row type before 1st/2nd header",
            ));
        }
        if c0.is_empty() {
            return Err(IngestError::row(
                file,
                *line,
                content,
                "unexpected continuation; expected jurisdiction A row",
            ));
        }
        if c1 != "A" {
            return Err(IngestError::row(
                file,
                *line,
                content,
                format!("unknown row type, expected A, got {c1:?}"),
            ));
        }
        let code = cra_code_to_jurisdiction(c0)?;
        if i + 2 >= rows.len() {
            return Err(IngestError::row(
                file,
                *line,
                content,
                format!("truncated group for {code}: expected 3 lines"),
            ));
        }
        let rate_row = &rows[i + 1];
        let const_row = &rows[i + 2];
        if is_empty_row(&rate_row.1) || is_empty_row(&const_row.1) {
            return Err(IngestError::row(
                file,
                *line,
                content,
                format!("truncated group for {code}: expected 3 lines"),
            ));
        }
        let rate_kind = cell(&rate_row.1, 1).trim();
        let const_kind = cell(&const_row.1, 1).trim();
        let expected_rate = if code == "FED" { "R" } else { "V" };
        let expected_const = if code == "FED" { "K" } else { "KP" };
        if cell(&rate_row.1, 0).trim() != "" || rate_kind != expected_rate {
            return Err(IngestError::row(
                file,
                rate_row.0,
                &rate_row.2,
                format!("truncated or unknown rate row for {code}"),
            ));
        }
        if cell(&const_row.1, 0).trim() != "" || const_kind != expected_const {
            return Err(IngestError::row(
                file,
                const_row.0,
                &const_row.2,
                format!("truncated or unknown constant row for {code}"),
            ));
        }
        let brackets = zip_brackets(file, code, fields, &rate_row.1, &const_row.1, *line)?;
        out.insert(code.to_string(), brackets);
        i += 3;
    }
    Ok(out)
}

fn zip_brackets(
    file: &str,
    code: &str,
    a_row: &[String],
    r_row: &[String],
    k_row: &[String],
    line: usize,
) -> Result<Vec<Bracket>, IngestError> {
    let mut brackets = Vec::new();
    let max_len = a_row.len().max(r_row.len()).max(k_row.len());
    for col in 2..max_len {
        let a = cell(a_row, col).trim();
        let r = cell(r_row, col).trim();
        let k = cell(k_row, col).trim();
        if a.is_empty() && r.is_empty() && k.is_empty() {
            continue;
        }
        if a.is_empty() || r.is_empty() || k.is_empty() {
            return Err(IngestError::row(
                file,
                line,
                &format!("{a},{r},{k}"),
                format!("{code}: incomplete bracket column {col}"),
            ));
        }
        let keep_zero = brackets.is_empty();
        brackets.push(Bracket {
            threshold: as_money(a, keep_zero)?,
            rate: as_rate(r, 4)?,
            constant: as_money(k, false)?,
            prorated: false,
        });
    }
    if brackets.is_empty() {
        return Err(IngestError::message(format!(
            "{file}: {code} has no brackets"
        )));
    }
    Ok(brackets)
}

pub fn parse_other_amounts(
    file: &str,
    text: &str,
) -> Result<BTreeMap<String, OtherAmounts>, IngestError> {
    let rows = csv_rows(file, text)?;
    let mut out: BTreeMap<String, OtherAmounts> = BTreeMap::new();
    let mut headers: Option<Vec<String>> = None;
    let mut current: Option<String> = None;
    for (line, fields, content) in &rows {
        if is_empty_row(fields) {
            continue;
        }
        let c0 = cell(fields, 0).trim();
        if c0.starts_with("Table") {
            continue;
        }
        if c0.starts_with('*') {
            continue;
        }
        if headers.is_none() {
            if cell(fields, 1).trim() != "Basic amount" {
                return Err(IngestError::row(
                    file,
                    *line,
                    content,
                    "unknown column header; expected Basic amount",
                ));
            }
            headers = Some(fields.iter().map(|s| s.trim().to_string()).collect());
            continue;
        }
        let hdr = headers.as_ref().unwrap();
        for (idx, name) in hdr.iter().enumerate().skip(1) {
            if name.is_empty() {
                continue;
            }
            let expected = [
                "Basic amount",
                "Index rate",
                "LCP rate",
                "LCP amount",
                "CEA",
                "S2",
                "T4 to V1",
                "V1 rate",
                "Abatement",
                "Surtax",
            ];
            if !expected.contains(&name.as_str()) {
                return Err(IngestError::row(
                    file,
                    *line,
                    content,
                    format!("unknown column {name:?}"),
                ));
            }
            let _ = idx;
        }
        if !c0.is_empty() {
            let code = cra_code_to_jurisdiction(c0)?.to_string();
            current = Some(code.clone());
            let entry = out.entry(code.clone()).or_default();
            fill_other(entry, hdr, fields)?;
        } else {
            let code = current.as_deref().ok_or_else(|| {
                IngestError::row(
                    file,
                    *line,
                    content,
                    "continuation row with no jurisdiction",
                )
            })?;
            let entry = out.get_mut(code).ok_or_else(|| {
                IngestError::row(
                    file,
                    *line,
                    content,
                    "continuation for unknown jurisdiction",
                )
            })?;
            fill_surtax_continuation(entry, hdr, fields)?;
        }
    }
    Ok(out)
}

fn col_index(hdr: &[String], name: &str) -> Option<usize> {
    hdr.iter().position(|h| h == name)
}

fn fill_other(
    entry: &mut OtherAmounts,
    hdr: &[String],
    fields: &[String],
) -> Result<(), IngestError> {
    if let Some(i) = col_index(hdr, "Basic amount") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.basic = Some(if let Some(tok) = special_token(v) {
                BasicCell::Token(tok)
            } else {
                BasicCell::Amount(as_money(v, false)?)
            });
        }
    }
    if let Some(i) = col_index(hdr, "Index rate") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.index_rate = Some(as_rate(v, 3)?);
        }
    }
    if let Some(i) = col_index(hdr, "LCP rate") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.lcp_rate = Some(as_rate(v, 3)?);
        }
    }
    if let Some(i) = col_index(hdr, "LCP amount") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.lcp_amount = Some(as_money(v, false)?);
        }
    }
    if let Some(i) = col_index(hdr, "CEA") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.cea = Some(as_money(v, false)?);
        }
    }
    if let Some(i) = col_index(hdr, "S2") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.s2 = Some(as_money(v, false)?);
        }
    }
    fill_surtax_continuation(entry, hdr, fields)?;
    if let Some(i) = col_index(hdr, "Abatement") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.abatement = Some(as_rate(v, 3)?);
        }
    }
    if let Some(i) = col_index(hdr, "Surtax") {
        let v = cell(fields, i).trim();
        if !is_absent(v) {
            entry.surtax_flat = Some(as_rate(v, 2)?);
        }
    }
    Ok(())
}

fn fill_surtax_continuation(
    entry: &mut OtherAmounts,
    hdr: &[String],
    fields: &[String],
) -> Result<(), IngestError> {
    let t = col_index(hdr, "T4 to V1").map(|i| cell(fields, i).trim().to_string());
    let r = col_index(hdr, "V1 rate").map(|i| cell(fields, i).trim().to_string());
    match (t.as_deref(), r.as_deref()) {
        (Some(t), Some(r)) if !is_absent(t) && !is_absent(r) => {
            if t == "0" && (r == "0" || r == "0.0" || r == "0.00") {
                return Ok(());
            }
            entry.surtax.push((as_money(t, false)?, as_rate(r, 2)?));
        }
        (Some(t), Some(r)) if is_absent(t) && is_absent(r) => {}
        (Some(_), Some(_)) => {
            return Err(IngestError::message(
                "T4 to V1 / V1 rate pair is incomplete",
            ));
        }
        _ => {}
    }
    Ok(())
}

fn find_name<'a>(names: &'a [&str], prefix: &str) -> Result<&'a str, IngestError> {
    names
        .iter()
        .copied()
        .find(|n| n.starts_with(prefix))
        .ok_or_else(|| IngestError::message(format!("archive missing {prefix}* csv")))
}

fn parse_cpp(dir: &Path, names: &[&str]) -> Result<Value, IngestError> {
    let ttl = parse_cpp_row(dir, find_name(names, "cpp-qpp-ttl-")?, "YMPE")?;
    let br = parse_cpp_row(dir, find_name(names, "cpp-qpp-br-")?, "base")?;
    let add = parse_cpp_row(dir, find_name(names, "cpp-qpp-addntl-")?, "add")?;
    let scnd = parse_cpp_row(dir, find_name(names, "cpp-qpp-scnd-addntl-")?, "scnd")?;
    Ok(json!({
        "ympe": ttl.ympe,
        "yampe": scnd.yampe,
        "basic_exemption": ttl.basic_exemption,
        "total_rate": ttl.total_rate,
        "total_max": ttl.total_max,
        "base_rate": br.base_rate,
        "base_max": br.base_max,
        "first_additional_rate": add.add_rate,
        "first_additional_max": add.add_max,
        "second_additional_rate": scnd.scnd_rate,
        "second_additional_max": scnd.scnd_max,
    }))
}

#[derive(Default)]
struct CppRow {
    ympe: String,
    yampe: String,
    basic_exemption: String,
    total_rate: String,
    total_max: String,
    base_rate: String,
    base_max: String,
    add_rate: String,
    add_max: String,
    scnd_rate: String,
    scnd_max: String,
}

fn parse_cpp_row(dir: &Path, name: &str, kind: &str) -> Result<CppRow, IngestError> {
    let (_path, bytes) = read_verified(dir, name)?;
    let text = decode_bytes(&bytes).text;
    let rows = csv_rows(name, &text)?;
    let data = rows
        .iter()
        .find(|(_, f, _)| cell(f, 0).starts_with("CPP (Canada except QC)"))
        .ok_or_else(|| {
            IngestError::message(format!("{name}: missing CPP (Canada except QC) row"))
        })?;
    let f = &data.1;
    let mut row = CppRow::default();
    match kind {
        "YMPE" => {
            row.ympe = as_money(cell(f, 1), false)?;
            row.basic_exemption = as_money(cell(f, 2), false)?;
            row.total_rate = as_rate(cell(f, 4), 4)?;
            row.total_max = as_money(cell(f, 5), false)?;
        }
        "base" => {
            row.base_rate = as_rate(cell(f, 2), 4)?;
            row.base_max = as_money(cell(f, 3), false)?;
        }
        "add" => {
            row.add_rate = as_rate(cell(f, 2), 4)?;
            row.add_max = as_money(cell(f, 3), false)?;
        }
        "scnd" => {
            row.yampe = as_money(cell(f, 2), false)?;
            row.scnd_rate = as_rate(cell(f, 4), 4)?;
            row.scnd_max = as_money(cell(f, 5), false)?;
        }
        _ => return Err(IngestError::message("internal cpp kind")),
    }
    Ok(row)
}

fn parse_ei(dir: &Path, names: &[&str]) -> Result<Value, IngestError> {
    let name = find_name(names, "ei-")?;
    let (_path, bytes) = read_verified(dir, name)?;
    let text = decode_bytes(&bytes).text;
    let rows = csv_rows(name, &text)?;
    let data = rows
        .iter()
        .find(|(_, f, _)| cell(f, 0).trim() == "Canada except QC")
        .ok_or_else(|| IngestError::message(format!("{name}: missing Canada except QC row")))?;
    let f = &data.1;
    Ok(json!({
        "max_insurable": as_money(cell(f, 1), false)?,
        "employee_rate": as_rate(cell(f, 2), 4)?,
        "employer_rate": as_rate(cell(f, 3), 5)?,
        "employee_max": as_money(cell(f, 4), false)?,
        "employer_max": as_money(cell(f, 5), false)?,
    }))
}

fn parse_qpip(dir: &Path, names: &[&str]) -> Result<Value, IngestError> {
    let name = find_name(names, "qpip-")?;
    let (_path, bytes) = read_verified(dir, name)?;
    let text = decode_bytes(&bytes).text;
    let rows = csv_rows(name, &text)?;
    let data = rows
        .iter()
        .find(|(_, f, _)| cell(f, 0).trim() == "QC")
        .ok_or_else(|| IngestError::message(format!("{name}: missing QC row")))?;
    let f = &data.1;
    Ok(json!({
        "max_insurable": as_money(cell(f, 1), false)?,
        "employee_rate": as_rate(cell(f, 2), 5)?,
        "employer_rate": as_rate(cell(f, 3), 5)?,
        "employee_max": as_money(cell(f, 4), false)?,
        "employer_max": as_money(cell(f, 5), false)?,
    }))
}

fn parse_claim_table(dir: &Path, name: &str) -> Result<(String, ClaimTable), IngestError> {
    let code = filename_to_jurisdiction(name)?.to_string();
    let (_path, bytes) = read_verified(dir, name)?;
    let text = decode_bytes(&bytes).text;
    let rows = csv_rows(name, &text)?;
    let mut title = String::new();
    let mut parsed = Vec::new();
    for (line, fields, content) in &rows {
        let c0 = cell(fields, 0).trim();
        if c0.starts_with("Table") {
            title = c0.to_string();
            continue;
        }
        if c0 == "Claim code" || c0.is_empty() {
            continue;
        }
        if special_token(c0).is_some() {
            continue;
        }
        let from = cell(fields, 1).trim();
        let to = cell(fields, 2).trim();
        let tcp_raw = cell(fields, 3).trim();
        let k1p_raw = cell(fields, 4).trim();
        if c0 == "0" {
            if special_token(from) != Some(SpecialToken::NoClaimAmount) {
                return Err(IngestError::row(
                    name,
                    *line,
                    content,
                    "claim code 0 must be No claim amount",
                ));
            }
            parsed.push(ClaimCodeRow {
                code: c0.to_string(),
                claim_from: from.to_string(),
                claim_to: to.to_string(),
                tcp: as_money(tcp_raw, false)?,
                k1p: as_money(k1p_raw, false)?,
            });
            continue;
        }
        if special_token(tcp_raw).is_some() {
            return Err(IngestError::row(
                name,
                *line,
                content,
                "claim code TC is a token, not an amount",
            ));
        }
        parsed.push(ClaimCodeRow {
            code: c0.to_string(),
            claim_from: from.to_string(),
            claim_to: to.to_string(),
            tcp: as_money(tcp_raw, false)?,
            k1p: as_money(k1p_raw, false)?,
        });
    }
    if parsed.iter().all(|r| r.code != "1") {
        return Err(IngestError::message(format!(
            "{name}: missing claim code 1 row"
        )));
    }
    Ok((
        code,
        ClaimTable {
            title,
            rows: parsed,
        },
    ))
}
