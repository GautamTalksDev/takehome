//! Lexical number movement. No numeric parse — only character surgery.

use crate::error::IngestError;

/// Strip CRA thousands separators from a quoted decimal field.
///
/// `"1,123.07"` → `"1123.07"`. Rejects `"1,12.07"` and `"1,,123.07"` rather
/// than repairing them. Does not parse the value.
pub fn strip_thousands(raw: &str) -> Result<String, IngestError> {
    let trimmed = raw.trim().trim_end_matches('*').trim();
    if trimmed.is_empty() {
        return Err(IngestError::message(format!(
            "empty numeric field from {raw:?}"
        )));
    }
    if matches!(trimmed, "–" | "-" | "—" | ".") {
        return Err(IngestError::message(format!(
            "not a numeric field: {trimmed:?}"
        )));
    }
    if !trimmed.contains(',') {
        if !is_plain_decimal(trimmed) {
            return Err(IngestError::message(format!(
                "not a lexical decimal: {trimmed:?}"
            )));
        }
        return Ok(trimmed.to_string());
    }
    let (int_part, frac) = match trimmed.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (trimmed, None),
    };
    if let Some(f) = frac {
        if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) || f.contains(',') {
            return Err(IngestError::message(format!(
                "invalid fractional part in {trimmed:?}"
            )));
        }
    }
    let groups: Vec<&str> = int_part.split(',').collect();
    if groups.len() < 2 || groups.iter().any(|g| g.is_empty()) {
        return Err(IngestError::message(format!(
            "invalid thousands grouping in {trimmed:?}"
        )));
    }
    for (i, group) in groups.iter().enumerate() {
        if !group.chars().all(|c| c.is_ascii_digit()) {
            return Err(IngestError::message(format!(
                "non-digit in thousands grouping {trimmed:?}"
            )));
        }
        if i == 0 {
            if group.is_empty() || group.len() > 3 {
                return Err(IngestError::message(format!(
                    "invalid thousands grouping in {trimmed:?}"
                )));
            }
        } else if group.len() != 3 {
            return Err(IngestError::message(format!(
                "invalid thousands grouping in {trimmed:?}"
            )));
        }
    }
    let mut out = groups.concat();
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    Ok(out)
}

fn is_plain_decimal(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_digit() {
        return false;
    }
    let mut seen_dot = false;
    for c in chars {
        if c == '.' {
            if seen_dot {
                return false;
            }
            seen_dot = true;
        } else if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

/// Pad a lexical decimal's fractional part with zeros up to `places`.
/// Never truncates extra digits. Never parses.
pub fn pad_fraction(raw: &str, places: usize) -> String {
    match raw.split_once('.') {
        None => {
            if places == 0 {
                raw.to_string()
            } else {
                format!("{raw}.{}", "0".repeat(places))
            }
        }
        Some((_, frac)) if frac.len() >= places => raw.to_string(),
        Some((int, frac)) => format!("{int}.{frac}{}", "0".repeat(places - frac.len())),
    }
}

/// Money field: two fractional digits, except a bare threshold `"0"` stays `"0"`.
pub fn as_money(raw: &str, keep_bare_zero: bool) -> Result<String, IngestError> {
    let stripped = strip_thousands(raw)?;
    if keep_bare_zero && stripped == "0" {
        return Ok(stripped);
    }
    Ok(pad_fraction(&stripped, 2))
}

/// Rate field: pad to `places` but keep any extra published digits.
pub fn as_rate(raw: &str, places: usize) -> Result<String, IngestError> {
    let stripped = strip_thousands(raw)?;
    Ok(pad_fraction(&stripped, places))
}
