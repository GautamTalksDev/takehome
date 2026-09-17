//! Tests 32–35: deterministic versioned grid, boundary search, provenance, coverage.

use std::collections::BTreeMap;
use takehome_core::request::Province;
use takehome_core::rules::loader::EMBEDDED_REGISTRY;
use takehome_core::rules::schema::{
    BasicPersonalAmount, CalculationOption, JurisdictionCode, OptionScoped,
};
use takehome_grid_gen::{
    canonical_digest, generate, rule_set_neighborhood_dates, BC_TAX_REDUCTION_START,
    BONUS_SHORTCUT, GRID_JURISDICTIONS, GRID_VERSION, SEED,
};

fn mentions(grid: &takehome_grid_gen::Grid, province: Province, needle: &str) -> bool {
    let code = province.as_str();
    grid.cases.iter().any(|case| {
        case.province == code
            && (case.targets.iter().any(|t| t.contains(needle))
                || case.gross_pay == needle
                || case.as_of == needle
                || case.bonus.as_deref() == Some(needle))
    })
}

/// Test 32 — same version, byte-identical output.
#[test]
fn grid_is_deterministic_byte_identical() {
    let a = generate().expect("grid a");
    let b = generate().expect("grid b");
    assert_eq!(a.manifest.grid_version, GRID_VERSION);
    assert_eq!(a.manifest.seed, SEED);
    assert_eq!(a, b);
    assert_eq!(canonical_digest(&a).unwrap(), canonical_digest(&b).unwrap());
}

/// Test 33 — every listed boundary appears for every jurisdiction that has it.
#[test]
fn every_listed_boundary_appears_in_the_grid() {
    let grid = generate().expect("grid");
    let jan = EMBEDDED_REGISTRY
        .resolve("2026-01-01".parse().unwrap())
        .unwrap();
    let july = EMBEDDED_REGISTRY
        .resolve("2026-07-01".parse().unwrap())
        .unwrap();

    let fed = jan
        .jurisdictions
        .get(&JurisdictionCode("FED".to_string()))
        .unwrap();
    for province in GRID_JURISDICTIONS {
        for bracket in fed.brackets.get(CalculationOption::Option1) {
            let amount = bracket.threshold.to_string();
            assert!(
                mentions(&grid, *province, &amount),
                "{province} missing federal bracket {amount}"
            );
        }
        assert!(
            mentions(&grid, *province, &jan.cpp.ympe.to_string()),
            "{province} missing YMPE"
        );
        assert!(
            mentions(&grid, *province, &jan.cpp.yampe.to_string()),
            "{province} missing YAMPE"
        );
        if let BasicPersonalAmount::Dynamic {
            phaseout_start,
            phaseout_end,
            ..
        } = fed.basic_personal_amount.get(CalculationOption::Option1)
        {
            assert!(
                mentions(&grid, *province, &phaseout_start.to_string()),
                "{province} missing BPAF start"
            );
            assert!(
                mentions(&grid, *province, &phaseout_end.to_string()),
                "{province} missing BPAF end"
            );
        }
        assert!(
            mentions(&grid, *province, BONUS_SHORTCUT),
            "{province} missing $5,000 bonus shortcut"
        );
        for date in rule_set_neighborhood_dates(&EMBEDDED_REGISTRY).unwrap() {
            assert!(
                mentions(&grid, *province, &date.to_string()),
                "{province} missing rule-set date {date}"
            );
        }
    }

    let on = jan
        .jurisdictions
        .get(&JurisdictionCode("ON".to_string()))
        .unwrap();
    for bracket in on.brackets.get(CalculationOption::Option1) {
        assert!(
            mentions(&grid, Province::On, &bracket.threshold.to_string()),
            "ON missing provincial bracket {}",
            bracket.threshold
        );
    }
    for tier in on.surtax.as_ref().unwrap() {
        assert!(
            mentions(&grid, Province::On, &tier.threshold.to_string()),
            "ON missing surtax {}",
            tier.threshold
        );
    }
    for tier in on.health_premium.as_ref().unwrap() {
        assert!(
            mentions(&grid, Province::On, &tier.threshold.to_string()),
            "ON missing health premium {}",
            tier.threshold
        );
    }

    assert!(
        mentions(&grid, Province::Bc, BC_TAX_REDUCTION_START),
        "BC missing S start 25570"
    );
    let bc_jan = jan
        .jurisdictions
        .get(&JurisdictionCode("BC".to_string()))
        .unwrap();
    let bc_july = july
        .jurisdictions
        .get(&JurisdictionCode("BC".to_string()))
        .unwrap();
    let jan_upper = bc_jan
        .tax_reduction
        .as_ref()
        .unwrap()
        .get(CalculationOption::Option1)
        .dependant;
    assert!(
        mentions(&grid, Province::Bc, &jan_upper.to_string()),
        "BC missing January S upper {jan_upper}"
    );
    match bc_july.tax_reduction.as_ref().unwrap() {
        OptionScoped::PerOption { option1, option2 } => {
            assert!(mentions(
                &grid,
                Province::Bc,
                &option1.dependant.to_string()
            ));
            assert!(mentions(
                &grid,
                Province::Bc,
                &option2.dependant.to_string()
            ));
        }
        OptionScoped::Both(red) => {
            assert!(mentions(&grid, Province::Bc, &red.dependant.to_string()));
        }
    }

    let mb = jan
        .jurisdictions
        .get(&JurisdictionCode("MB".to_string()))
        .unwrap();
    if let BasicPersonalAmount::Dynamic {
        phaseout_start,
        phaseout_end,
        ..
    } = mb.basic_personal_amount.get(CalculationOption::Option1)
    {
        assert!(
            mentions(&grid, Province::Mb, &phaseout_start.to_string()),
            "MB missing BPAMB start"
        );
        assert!(
            mentions(&grid, Province::Mb, &phaseout_end.to_string()),
            "MB missing BPAMB end"
        );
    }

    assert!(
        !grid.cases.iter().any(|c| c.province == "QC"),
        "Quebec must not appear in the grid"
    );
}

/// Test 34 — emission carries version, git sha, case count; conformance records the version.
#[test]
fn grid_emission_carries_version_sha_count_and_conformance_records_it() {
    let grid = generate().expect("grid");
    assert_eq!(grid.manifest.grid_version, GRID_VERSION);
    assert_eq!(grid.manifest.seed, SEED);
    assert!(!grid.manifest.git_sha.is_empty());
    assert_eq!(
        grid.manifest.case_count,
        u64::try_from(grid.cases.len()).unwrap()
    );
    assert!(grid.manifest.case_count >= 250_000);

    let conformance = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CONFORMANCE.md"));
    assert!(
        conformance.contains(GRID_VERSION),
        "CONFORMANCE.md must record the grid_version this run used"
    );
    assert!(
        conformance.contains("grid_version"),
        "CONFORMANCE.md must name the grid_version field"
    );
}

/// Test 35 — ≥250k cases, every jurisdiction, counts within a factor of two of the mean.
#[test]
fn grid_covers_every_jurisdiction_evenly() {
    let grid = generate().expect("grid");
    assert!(
        grid.manifest.case_count >= 250_000,
        "need ≥250,000 cases, got {}",
        grid.manifest.case_count
    );
    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for case in &grid.cases {
        *counts.entry(case.province.as_str()).or_insert(0) += 1;
    }
    for province in GRID_JURISDICTIONS {
        assert!(
            counts.get(province.as_str()).copied().unwrap_or(0) > 0,
            "missing jurisdiction {}",
            province.as_str()
        );
    }
    assert_eq!(counts.len(), GRID_JURISDICTIONS.len());
    assert!(!counts.contains_key("QC"));
    let n = u64::try_from(counts.len()).unwrap();
    let total: u64 = counts.values().copied().sum();
    let mean = total / n;
    for (code, count) in &counts {
        assert!(
            *count * 2 >= mean && mean * 2 >= *count,
            "{code} has {count} cases; mean {mean} (factor-of-two rule, spec §4.1)"
        );
    }
}

/// Identity sample is 500 cases, every employment jurisdiction, deterministic.
#[test]
fn wasm_identity_sample_is_500_across_thirteen_and_deterministic() {
    let a = takehome_grid_gen::wasm_identity_sample(500).expect("sample a");
    let b = takehome_grid_gen::wasm_identity_sample(500).expect("sample b");
    assert_eq!(a, b);
    assert_eq!(a.requests.len(), 500);
    assert_eq!(a.ids.len(), 500);
    assert_eq!(a.jurisdictions.len(), GRID_JURISDICTIONS.len());
    let mut seen = std::collections::BTreeSet::new();
    for req in &a.requests {
        seen.insert(req.province.as_str());
    }
    assert_eq!(seen.len(), GRID_JURISDICTIONS.len());
    assert!(!seen.contains("QC"));
    for req in &a.requests {
        takehome_core::calculate(req, &EMBEDDED_REGISTRY)
            .unwrap_or_else(|e| panic!("{}: {e}", req.province.as_str()));
    }
}
