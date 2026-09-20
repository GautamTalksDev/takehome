//! Find exact half-cent T2/P forms where IEEE float residue ≠ decimal half-up.
//!
//! Intentionally uses `f64` to prove IEEE half-up diverges from decimal. The
//! float-ban script does not scan `tests/`; clippy must allow the arithmetic.
#![allow(clippy::float_arithmetic)]
use rust_decimal::Decimal;
use std::str::FromStr;
use takehome_core::decimal::Money;
use takehome_core::rules::loader::load_embedded_registry;
use takehome_core::{calculate, Request};

fn is_exact_half_cent(q: &str) -> bool {
    let d = Decimal::from_str(q).unwrap();
    let scaled = d * Decimal::from(100);
    let frac = scaled - scaled.trunc();
    frac.abs() == Decimal::from_str("0.5").unwrap()
}

fn float_half_up(t2: &str, p: u16) -> String {
    let t2_f = f64::from_str(t2).unwrap();
    let q = t2_f / f64::from(p);
    Decimal::from_f64_retain(q)
        .unwrap()
        .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
        .to_string()
}

#[test]
fn emit_discriminating_probes() {
    let queue_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/grids/pdoc-queue.json");
    // `data/grids/**` is gitignored — this probe generator is a local tool, not CI.
    if !queue_path.is_file() {
        eprintln!(
            "skip emit_discriminating_probes: missing {}",
            queue_path.display()
        );
        return;
    }
    let reg = load_embedded_registry().unwrap();
    let queue: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(&queue_path).unwrap()).unwrap();

    let mut out: Vec<serde_json::Value> = Vec::new();
    for item in &queue {
        let prov = item["province"].as_str().unwrap();
        if prov == "AB" || prov == "OutsideCanada" {
            continue;
        }
        let p = u16::try_from(item["pay_period"].as_u64().unwrap()).unwrap();
        let gross = item["gross_pay"].as_str().unwrap();
        let fc = u8::try_from(item["federal_claim_code"].as_u64().unwrap()).unwrap();
        let pc = u8::try_from(item["provincial_claim_code"].as_u64().unwrap()).unwrap();
        let req: Request = serde_json::from_value(serde_json::json!({
            "as_of": "2026-07-01",
            "province": prov,
            "pay_period": p,
            "gross_pay": gross,
            "federal_claim_code": fc,
            "provincial_claim_code": pc,
            "calculation_option": "option1"
        }))
        .unwrap();
        let Ok(resp) = calculate(&req, &reg) else {
            continue;
        };
        let p_m = Money::parse(&p.to_string()).unwrap();
        let t2q = resp.breakdown.t2.checked_div(p_m).unwrap().to_string();
        if !is_exact_half_cent(&t2q) {
            continue;
        }
        let eng = resp.employee.provincial_tax.to_string();
        let pred = float_half_up(&resp.breakdown.t2.to_string(), p);
        if eng == pred {
            continue;
        }
        out.push(serde_json::json!({
            "province": prov,
            "pay_period": p,
            "gross_pay": gross,
            "federal_claim_code": fc,
            "provincial_claim_code": pc,
            "t2": resp.breakdown.t2.to_string(),
            "engine_provincial": eng,
            "float_pred": pred,
            "federal": resp.employee.federal_tax.to_string(),
        }));
    }

    // Dedup identical forms (the 20-probe run listed NL 4333.33 twice).
    let mut seen: std::collections::BTreeSet<(String, u64, String, u64, u64)> =
        std::collections::BTreeSet::new();
    out.retain(|c| {
        seen.insert((
            c["province"].as_str().unwrap().to_string(),
            c["pay_period"].as_u64().unwrap(),
            c["gross_pay"].as_str().unwrap().to_string(),
            c["federal_claim_code"].as_u64().unwrap(),
            c["provincial_claim_code"].as_u64().unwrap(),
        ))
    });

    // Stratify: take every unique NL form first (the 2/20 float hits were one
    // NL gross), then up to 6 per other province, cap 50.
    let mut by: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
    for c in out {
        by.entry(c["province"].as_str().unwrap().to_string())
            .or_default()
            .push(c);
    }
    let mut picked = Vec::new();
    if let Some(mut nl) = by.remove("NL") {
        nl.sort_by(|a, b| {
            a["pay_period"]
                .as_u64()
                .cmp(&b["pay_period"].as_u64())
                .then(a["gross_pay"].as_str().cmp(&b["gross_pay"].as_str()))
        });
        picked.extend(nl);
    }
    for (_p, mut rows) in by {
        rows.sort_by(|a, b| {
            a["pay_period"]
                .as_u64()
                .cmp(&b["pay_period"].as_u64())
                .then(a["gross_pay"].as_str().cmp(&b["gross_pay"].as_str()))
        });
        picked.extend(rows.into_iter().take(6));
    }
    picked.truncate(50);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/grids/halfcent-probes.json");
    std::fs::write(&path, serde_json::to_string_pretty(&picked).unwrap() + "\n").unwrap();
    println!("wrote {} probes → {}", picked.len(), path.display());
    for c in &picked {
        println!(
            "{} P={} gross={} eng={} float={}",
            c["province"], c["pay_period"], c["gross_pay"], c["engine_provincial"], c["float_pred"]
        );
    }
    assert!(
        picked.len() >= 40,
        "need ≥40 discriminating unique non-AB probes, got {}",
        picked.len()
    );
}
