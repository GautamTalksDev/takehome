//! Deterministic T4127 conformance grid (spec §11.2).
//!
//! Same [`SEED`], same [`GRID_VERSION`], same embedded rules → byte-identical
//! output, forever. Boundary density beats volume.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use netpay_core::decimal::{DecimalError, Money, Rate};
use netpay_core::request::{ClaimCode, PayPeriod, Province, Request};
use netpay_core::rounding::round_tax_to_cent;
use netpay_core::rules::registry::Registry;
use netpay_core::rules::schema::{
    BasicPersonalAmount, CalculationOption, CalendarDate, DateParseError, Jurisdiction,
    JurisdictionCode, OptionScoped, RuleSet,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use thiserror::Error;

pub mod sampling;

/// Published grid identity. Conformance runs record this string.
pub const GRID_VERSION: &str = "2026.1";
/// Documented seed. The grid is a pure function of version + rules; the seed is
/// pinned so a future fill cannot drift.
pub const SEED: &str = "t4127-grid-2026.1";
/// T4127 Chapter 4 bonus / retro shortcut threshold.
pub const BONUS_SHORTCUT: &str = "5000.00";
/// BC S phase-out start (T4127 Chapter 4; not a JSON field).
pub const BC_TAX_REDUCTION_START: &str = "25570.00";
/// Geometric step for the log-spaced interior ladder.
const LOG_STEP: &str = "1.10";
const CENT: &str = "0.01";
const CLAIM_CODES: &[u8] = &[0, 1];

/// Thirteen employment jurisdictions. Quebec is excluded: it is not calculated.
pub const GRID_JURISDICTIONS: &[Province] = &[
    Province::Ab,
    Province::Bc,
    Province::Mb,
    Province::Nb,
    Province::Nl,
    Province::Ns,
    Province::Nt,
    Province::Nu,
    Province::On,
    Province::Pe,
    Province::Sk,
    Province::Yt,
    Province::OutsideCanada,
];

/// Grid generation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GridError {
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    #[error("{0}")]
    Message(String),
}

/// Header emitted with every grid and copied onto every conformance run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridManifest {
    pub grid_version: String,
    pub seed: String,
    pub git_sha: String,
    pub case_count: u64,
}

/// One grid cell. `targets` names the spec §11.2 boundaries this cell is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GridCase {
    pub id: String,
    pub as_of: String,
    pub province: String,
    pub pay_period: u16,
    pub gross_pay: String,
    pub calculation_option: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonus: Option<String>,
    pub federal_claim_code: u8,
    pub provincial_claim_code: u8,
    pub targets: Vec<String>,
}

/// Versioned case list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Grid {
    pub manifest: GridManifest,
    pub cases: Vec<GridCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CaseKey {
    province: String,
    pay_period: u16,
    as_of: String,
    option: String,
    gross_pay: String,
    bonus: Option<String>,
    claim_code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NamedAmount {
    kind: String,
    amount: Money,
}

impl NamedAmount {
    fn tag(&self) -> String {
        format!("{}:{}", self.kind, self.amount)
    }
}

pub use sampling::{
    classify_case, is_boundary_target, is_pdoc_capturable_form, july_boundary_queue, oracle_census,
    oracle_class_for_case, pdoc_fingerprint, sampling_report, stratified_smoke_queue, CaseClass,
    CensusRow, OracleClass, PdocFingerprint, PdocQueueItem, SamplingReport,
    PDOC_CAPTURABLE_PAY_PERIODS, PDOC_UNCAPTURABLE_PAY_PERIODS,
};

/// Build the grid from the embedded rule registry.
pub fn generate() -> Result<Grid, GridError> {
    generate_with(&netpay_core::rules::loader::EMBEDDED_REGISTRY)
}

/// Build the grid from an explicit registry (tests / tooling).
pub fn generate_with(registry: &Registry) -> Result<Grid, GridError> {
    let dates = rule_set_neighborhood_dates(registry)?;
    let mut cells: BTreeMap<CaseKey, BTreeSet<String>> = BTreeMap::new();
    for province in GRID_JURISDICTIONS {
        for as_of in &dates {
            let set = set_for_ladder(registry, *as_of)?;
            let named = named_boundaries(set, *province)?;
            let annual_exact: Vec<Money> = named.iter().map(|n| n.amount).collect();
            let interiors = log_interiors(&unique_sorted(&annual_exact)?)?;
            for claim in CLAIM_CODES {
                for p in PayPeriod::LEGAL {
                    let pay_period =
                        PayPeriod::new(*p).map_err(|e| GridError::Message(e.to_string()))?;
                    for named in &named {
                        for annual in neighborhood(named.amount)? {
                            let period = annual_to_period(annual, pay_period)?;
                            for neighbor in period_neighborhood(period)? {
                                insert_cell(
                                    &mut cells,
                                    *province,
                                    pay_period,
                                    *as_of,
                                    CalculationOption::Option1,
                                    neighbor,
                                    None,
                                    *claim,
                                    named.tag(),
                                );
                            }
                        }
                    }
                    for annual in &interiors {
                        let period = annual_to_period(*annual, pay_period)?;
                        for neighbor in period_neighborhood(period)? {
                            insert_cell(
                                &mut cells,
                                *province,
                                pay_period,
                                *as_of,
                                CalculationOption::Option1,
                                neighbor,
                                None,
                                *claim,
                                format!("log_ladder:{annual}"),
                            );
                        }
                    }
                    for bonus in neighborhood(Money::parse(BONUS_SHORTCUT)?)? {
                        let gross = annual_to_period(Money::parse("52000.00")?, pay_period)?;
                        insert_cell(
                            &mut cells,
                            *province,
                            pay_period,
                            *as_of,
                            CalculationOption::Option1,
                            gross,
                            Some(bonus),
                            *claim,
                            format!("bonus_shortcut:{bonus}"),
                        );
                    }
                    insert_cell(
                        &mut cells,
                        *province,
                        pay_period,
                        *as_of,
                        CalculationOption::Option1,
                        annual_to_period(Money::parse("1000.00")?, pay_period)?,
                        None,
                        *claim,
                        format!("rule_set_date:{as_of}"),
                    );
                }
            }
        }
    }
    let cases: Vec<GridCase> = cells
        .into_iter()
        .map(|(key, targets)| GridCase {
            id: case_id(&key),
            as_of: key.as_of,
            province: key.province,
            pay_period: key.pay_period,
            gross_pay: key.gross_pay,
            calculation_option: key.option,
            bonus: key.bonus,
            federal_claim_code: key.claim_code,
            provincial_claim_code: key.claim_code,
            targets: targets.into_iter().collect(),
        })
        .collect();
    let case_count = u64::try_from(cases.len())
        .map_err(|_| GridError::Message("case count does not fit in u64".to_string()))?;
    Ok(Grid {
        manifest: GridManifest {
            grid_version: GRID_VERSION.to_string(),
            seed: SEED.to_string(),
            git_sha: git_sha().to_string(),
            case_count,
        },
        cases,
    })
}

/// Canonical bytes: compact manifest JSON, then one compact case JSON per line.
pub fn canonical_bytes(grid: &Grid) -> Result<Vec<u8>, GridError> {
    let mut out =
        serde_json::to_vec(&grid.manifest).map_err(|e| GridError::Message(e.to_string()))?;
    out.push(b'\n');
    for case in &grid.cases {
        let line = serde_json::to_vec(case).map_err(|e| GridError::Message(e.to_string()))?;
        out.extend_from_slice(&line);
        out.push(b'\n');
    }
    Ok(out)
}

/// SHA-256 of [`canonical_bytes`] — identity of a grid emission.
pub fn canonical_digest(grid: &Grid) -> Result<String, GridError> {
    let bytes = canonical_bytes(grid)?;
    Ok(hex_lower(Sha256::digest(&bytes)))
}

/// Git sha baked in at compile time.
pub fn git_sha() -> &'static str {
    env!("NETPAY_GRID_GIT_SHA")
}

/// Map a grid case onto a [`Request`] (claim codes, not fixed TD1 dollars).
pub fn case_to_request(case: &GridCase) -> Result<Request, GridError> {
    let as_of: CalendarDate = case
        .as_of
        .parse()
        .map_err(|e: DateParseError| GridError::Message(e.to_string()))?;
    let option = match case.calculation_option.as_str() {
        "option1" => CalculationOption::Option1,
        "option2" => CalculationOption::Option2,
        other => {
            return Err(GridError::Message(format!(
                "unknown calculation_option {other}"
            )))
        }
    };
    let province: Province = serde_json::from_str(&format!("\"{}\"", case.province))
        .map_err(|e| GridError::Message(e.to_string()))?;
    Ok(Request {
        as_of,
        province,
        pay_period: PayPeriod::new(case.pay_period)
            .map_err(|e| GridError::Message(e.to_string()))?,
        gross_pay: Money::parse(&case.gross_pay)?,
        calculation_option: option,
        pensionable_earnings: None,
        insurable_earnings: None,
        federal_claim_code: Some(ClaimCode::Code(case.federal_claim_code)),
        provincial_claim_code: Some(ClaimCode::Code(case.provincial_claim_code)),
        federal_tc: None,
        provincial_tcp: None,
        cpp_months: 12,
        k2_method: netpay_core::request::K2Method::PdocObserved,
        rounding_compat: netpay_core::request::RoundingCompat::T4127,
        ytd_pensionable_earnings: None,
        ytd_insurable_earnings: None,
        ytd_cpp: None,
        ytd_cpp2: None,
        ytd_ei: None,
        ytd_federal_tax: None,
        ytd_provincial_tax: None,
        pay_periods_elapsed: None,
        bonus: case.bonus.as_deref().map(Money::parse).transpose()?,
        retroactive_pay: None,
        commission_income: None,
        commission_expenses: None,
        estimated_annual_expenses: None,
        ytd_qpp: None,
        ytd_qpp2: None,
        ytd_qpip: None,
        prior_province: None,
        date_of_birth: None,
        cpp_exempt: None,
        cpp_election_after_65: None,
        additional_tax_requested: None,
        taxable_benefits: None,
        union_dues: None,
        alimony: None,
        child_care_expenses: None,
        prescribed_zone_deduction: None,
        lcf_purchase: None,
        lcp_purchase: None,
        dependants_under_19: None,
        dependants_disabled: None,
    })
}

/// Rule-set `effective_from` values and one civil day either side.
pub fn rule_set_neighborhood_dates(registry: &Registry) -> Result<Vec<CalendarDate>, GridError> {
    let mut dates = BTreeSet::new();
    for (_version, from, _to) in registry.versions() {
        dates.insert(prev_day(from)?);
        dates.insert(from);
        dates.insert(next_day(from)?);
    }
    Ok(dates.into_iter().collect())
}

fn set_for_ladder(registry: &Registry, as_of: CalendarDate) -> Result<&RuleSet, GridError> {
    match registry.resolve(as_of) {
        Ok(set) => Ok(set),
        Err(_) => {
            let versions = registry.versions();
            let first = versions
                .first()
                .ok_or_else(|| GridError::Message("empty registry".to_string()))?;
            registry
                .resolve(first.1)
                .map_err(|e| GridError::Message(e.to_string()))
        }
    }
}

fn named_boundaries(set: &RuleSet, province: Province) -> Result<Vec<NamedAmount>, GridError> {
    let mut out = BTreeSet::new();
    let fed = set
        .jurisdictions
        .get(&JurisdictionCode("FED".to_string()))
        .ok_or_else(|| GridError::Message("rule set has no FED".to_string()))?;
    push_brackets(&mut out, "federal_bracket", &fed.brackets);
    push_bpa(
        &mut out,
        "bpaf_phaseout",
        fed.basic_personal_amount.get(CalculationOption::Option1),
    );
    out.insert(NamedAmount {
        kind: "ympe".to_string(),
        amount: set.cpp.ympe,
    });
    out.insert(NamedAmount {
        kind: "yampe".to_string(),
        amount: set.cpp.yampe,
    });
    out.insert(NamedAmount {
        kind: "bonus_shortcut".to_string(),
        amount: Money::parse(BONUS_SHORTCUT)?,
    });
    if province != Province::OutsideCanada {
        let code = JurisdictionCode(province.as_str().to_string());
        let provincial = set
            .jurisdictions
            .get(&code)
            .ok_or_else(|| GridError::Message(format!("rule set has no {}", province.as_str())))?;
        push_brackets(&mut out, "provincial_bracket", &provincial.brackets);
        push_bpa(
            &mut out,
            "bpamb_phaseout",
            provincial
                .basic_personal_amount
                .get(CalculationOption::Option1),
        );
        if province == Province::On {
            if let Some(tiers) = provincial.surtax.as_ref() {
                for tier in tiers {
                    out.insert(NamedAmount {
                        kind: "ontario_surtax".to_string(),
                        amount: tier.threshold,
                    });
                }
            }
            if let Some(tiers) = provincial.health_premium.as_ref() {
                for tier in tiers {
                    out.insert(NamedAmount {
                        kind: "ontario_health_premium".to_string(),
                        amount: tier.threshold,
                    });
                }
            }
        }
        if province == Province::Bc {
            out.insert(NamedAmount {
                kind: "bc_tax_reduction".to_string(),
                amount: Money::parse(BC_TAX_REDUCTION_START)?,
            });
            push_bc_upper(&mut out, provincial);
        }
    }
    Ok(out.into_iter().collect())
}

fn push_brackets(
    out: &mut BTreeSet<NamedAmount>,
    kind: &str,
    brackets: &OptionScoped<Vec<netpay_core::rules::schema::Bracket>>,
) {
    for opt in [CalculationOption::Option1, CalculationOption::Option2] {
        for bracket in brackets.get(opt) {
            out.insert(NamedAmount {
                kind: kind.to_string(),
                amount: bracket.threshold,
            });
        }
    }
}

fn push_bpa(out: &mut BTreeSet<NamedAmount>, kind: &str, bpa: &BasicPersonalAmount) {
    if let BasicPersonalAmount::Dynamic {
        phaseout_start,
        phaseout_end,
        ..
    } = bpa
    {
        out.insert(NamedAmount {
            kind: kind.to_string(),
            amount: *phaseout_start,
        });
        out.insert(NamedAmount {
            kind: kind.to_string(),
            amount: *phaseout_end,
        });
    }
}

fn push_bc_upper(out: &mut BTreeSet<NamedAmount>, provincial: &Jurisdiction) {
    let Some(reduction) = provincial.tax_reduction.as_ref() else {
        return;
    };
    for opt in [CalculationOption::Option1, CalculationOption::Option2] {
        out.insert(NamedAmount {
            kind: "bc_tax_reduction".to_string(),
            amount: reduction.get(opt).dependant,
        });
    }
}

fn neighborhood(amount: Money) -> Result<Vec<Money>, GridError> {
    let cent = Money::parse(CENT)?;
    let mut values = vec![amount];
    values.push(amount.checked_add(cent)?);
    if amount > Money::ZERO {
        let lo = amount.checked_sub(cent)?;
        if !lo.is_negative() {
            values.push(lo);
        }
    }
    unique_sorted(&values)
}

fn period_neighborhood(period: Money) -> Result<Vec<Money>, GridError> {
    neighborhood(period)
}

fn unique_sorted(values: &[Money]) -> Result<Vec<Money>, GridError> {
    let mut set = BTreeSet::new();
    for value in values {
        set.insert(*value);
    }
    Ok(set.into_iter().collect())
}

fn log_interiors(sorted_boundaries: &[Money]) -> Result<Vec<Money>, GridError> {
    let step = Rate::parse(LOG_STEP)?;
    let one = Money::parse("1")?;
    let floor = Money::parse("1.00")?;
    let mut out = BTreeSet::new();
    for pair in sorted_boundaries.windows(2) {
        let lo = pair[0].max(floor);
        let hi = pair[1];
        if hi <= lo {
            continue;
        }
        let mut cur = lo;
        loop {
            let next = round_tax_to_cent(cur.checked_mul_rate(step)?.checked_div(one)?);
            if next <= cur || next >= hi {
                break;
            }
            out.insert(next);
            cur = next;
        }
    }
    Ok(out.into_iter().collect())
}

fn annual_to_period(annual: Money, pay_period: PayPeriod) -> Result<Money, GridError> {
    let p = Money::parse(&pay_period.get().to_string())?;
    Ok(round_tax_to_cent(annual.checked_div(p)?))
}

#[allow(clippy::too_many_arguments)]
fn insert_cell(
    cells: &mut BTreeMap<CaseKey, BTreeSet<String>>,
    province: Province,
    pay_period: PayPeriod,
    as_of: CalendarDate,
    option: CalculationOption,
    gross: Money,
    bonus: Option<Money>,
    claim_code: u8,
    target: String,
) {
    let option = match option {
        CalculationOption::Option1 => "option1",
        CalculationOption::Option2 => "option2",
    };
    let key = CaseKey {
        province: province.as_str().to_string(),
        pay_period: pay_period.get(),
        as_of: as_of.to_string(),
        option: option.to_string(),
        gross_pay: gross.to_string(),
        bonus: bonus.map(|b| b.to_string()),
        claim_code,
    };
    cells.entry(key).or_default().insert(target);
}

fn case_id(key: &CaseKey) -> String {
    let bonus = key.bonus.as_deref().unwrap_or("none");
    format!(
        "{}/{}/{}/{}/cc{}/g={}/b={bonus}",
        key.province, key.pay_period, key.as_of, key.option, key.claim_code, key.gross_pay
    )
}

fn next_day(d: CalendarDate) -> Result<CalendarDate, GridError> {
    if let Ok(next) = CalendarDate::new(d.year, d.month, d.day.saturating_add(1)) {
        return Ok(next);
    }
    if let Ok(next) = CalendarDate::new(d.year, d.month.saturating_add(1), 1) {
        return Ok(next);
    }
    CalendarDate::new(d.year.saturating_add(1), 1, 1).map_err(|e| GridError::Message(e.to_string()))
}

fn prev_day(d: CalendarDate) -> Result<CalendarDate, GridError> {
    if d.day > 1 {
        return CalendarDate::new(d.year, d.month, d.day - 1)
            .map_err(|e| GridError::Message(e.to_string()));
    }
    if d.month > 1 {
        let month = d.month - 1;
        for day in (1..=31).rev() {
            if let Ok(prev) = CalendarDate::new(d.year, month, day) {
                return Ok(prev);
            }
        }
    }
    let year = d
        .year
        .checked_sub(1)
        .ok_or_else(|| GridError::Message("calendar underflow".to_string()))?;
    for day in (1..=31).rev() {
        if let Ok(prev) = CalendarDate::new(year, 12, day) {
            return Ok(prev);
        }
    }
    Err(GridError::Message("calendar underflow".to_string()))
}

fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
