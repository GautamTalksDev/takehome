//! Spec §11.4 conformance report. A partial run is a document with no rate.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use takehome_core::request::Request;
use takehome_core::response::ENGINE_BUILD_SHA256;
use takehome_core::rules::loader::EMBEDDED_REGISTRY;
use takehome_core::{calculate, Money};
use takehome_grid_gen::{generate, oracle_census, sampling_report, CensusRow, SamplingReport};
use thiserror::Error;

pub mod markdown;
mod year;

const METHODOLOGY: &str = include_str!("methodology.md");
const M1_VECTORS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/takehome-core/tests/vectors/pdoc_ontario_2026_01.json"
));
const BONUS_VECTORS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/takehome-core/tests/vectors/pdoc_bonus_2026.json"
));
const GRID_PDOC_BOUNDARY_JSON: &str = include_str!("grid_pdoc_boundary_2026.json");

#[derive(Debug, Deserialize)]
struct GridPdocSnapshot {
    defined: u64,
    matched: u64,
    mismatched: u64,
    skipped_step2: u64,
    latest_pdoc_retrieved_at: String,
    browser: String,
    operator: String,
    pdoc_identity: String,
    disagreements: Vec<Disagreement>,
}

fn grid_pdoc_snapshot() -> Result<GridPdocSnapshot, ReportError> {
    Ok(serde_json::from_str(GRID_PDOC_BOUNDARY_JSON)?)
}

/// Overall document status. Incomplete PDOC corpora forbid an overall rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Complete,
    InProgress,
}

/// One measured corpus. Agreement rate is present only when the corpus is
/// complete *and* the oracle is PDOC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Corpus {
    pub id: String,
    pub oracle_class: String,
    pub cases_defined: u64,
    pub cases_measured: u64,
    pub pending: u64,
    pub exact_matches: u64,
    pub disagreements: Vec<Disagreement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_rate: Option<String>,
}

/// Engine vs PDOC on identically specified inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disagreement {
    pub id: String,
    pub request: serde_json::Value,
    pub engine: BTreeMap<String, String>,
    pub pdoc: BTreeMap<String, String>,
    pub delta: BTreeMap<String, String>,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdoc_believed_wrong: Option<String>,
}

/// Spec §11.4 machine-readable report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub status: RunStatus,
    pub grid_version: String,
    pub engine_build_sha256: String,
    pub rule_set_versions: Vec<String>,
    pub rule_set_sha256: String,
    pub pdoc_identity: String,
    pub browser: String,
    pub harness_git_sha: String,
    pub latest_pdoc_retrieved_at: String,
    pub sampling: SamplingReport,
    pub census: Vec<CensusRow>,
    pub corpora: Vec<Corpus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_rate: Option<String>,
    pub coverage_gaps: Vec<String>,
    pub methodology: Vec<String>,
}

/// Generator failure.
#[derive(Debug, Error)]
pub enum ReportError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Grid(#[from] takehome_grid_gen::GridError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Option 1 PDOC rate (M1 + July grid). The bonus class is named pending and
/// stays out of the denominator until amounts are captured from PDOC.
fn is_rate_bearing_pdoc(c: &Corpus) -> bool {
    c.oracle_class == "pdoc" && c.id != "pdoc-bonus-2026"
}

impl Corpus {
    fn from_counts(
        id: &str,
        oracle_class: &str,
        defined: u64,
        measured: u64,
        matches: u64,
        disagreements: Vec<Disagreement>,
    ) -> Self {
        let pending = defined.saturating_sub(measured);
        let complete = pending == 0 && measured == defined;
        let agreement_rate = if complete && oracle_class == "pdoc" && measured > 0 {
            Some(format!("{matches}/{measured}"))
        } else {
            None
        };
        Self {
            id: id.to_string(),
            oracle_class: oracle_class.to_string(),
            cases_defined: defined,
            cases_measured: measured,
            pending,
            exact_matches: matches,
            disagreements,
            agreement_rate,
        }
    }

    fn is_complete(&self) -> bool {
        self.pending == 0
    }

    /// Finished PDOC queue: step-2 skips are named, not pending, and not in the rate.
    /// Rate denominator is compared forms (`matched + mismatched`).
    fn from_pdoc_queue(
        id: &str,
        defined: u64,
        matched: u64,
        mismatched: u64,
        skipped_step2: u64,
        disagreements: Vec<Disagreement>,
    ) -> Self {
        let compared = matched.saturating_add(mismatched);
        let processed = compared.saturating_add(skipped_step2);
        let pending = defined.saturating_sub(processed);
        let complete = pending == 0 && processed == defined;
        let agreement_rate = if complete && compared > 0 {
            Some(format!("{matched}/{compared}"))
        } else {
            None
        };
        Self {
            id: id.to_string(),
            oracle_class: "pdoc".to_string(),
            cases_defined: defined,
            cases_measured: compared,
            pending,
            exact_matches: matched,
            disagreements,
            agreement_rate,
        }
    }
}

/// Workspace root (`tools/conformance/../..`).
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Build the report from the repository: M1 vectors, current grid, current rules.
pub fn report_from_repo() -> Result<Report, ReportError> {
    let grid = generate()?;
    let sampling = sampling_report(&grid, 0);
    let census = oracle_census(&grid);
    let m1 = measure_m1()?;
    let bonus = measure_bonus()?;
    let year = year::measure()?;
    let option2 = measure_option2()?;
    let pdoc_queue = measure_grid_pdoc(sampling.unique_form_boundary_july)?;
    let uncapturable = Corpus::from_counts(
        "grid-2026.1-uncapturable-boundary",
        "uncapturable",
        sampling.unique_form_boundary_july_uncapturable,
        0,
        0,
        Vec::new(),
    );
    let interiors = Corpus::from_counts(
        "grid-2026.1-interior-invariants",
        "invariants",
        sampling
            .by_class
            .get("log_ladder")
            .copied()
            .unwrap_or(0)
            .saturating_add(sampling.by_class.get("date_anchor").copied().unwrap_or(0)),
        0,
        0,
        Vec::new(),
    );
    let snap = grid_pdoc_snapshot()?;
    let mut report = assemble_report(
        sampling,
        census,
        vec![
            m1,
            bonus,
            year,
            option2,
            pdoc_queue,
            uncapturable,
            interiors,
        ],
        hash_rules(&workspace_root().join("data/rules"))?,
    );
    report.pdoc_identity = format!("2026-06-11 (M1); {} (grid queue)", snap.pdoc_identity);
    report.browser = snap.browser;
    report.harness_git_sha = snap.operator;
    report.latest_pdoc_retrieved_at = snap.latest_pdoc_retrieved_at;
    Ok(report)
}

/// Assemble a report from already-measured parts (tests / `--from-repo`).
pub fn assemble_report(
    sampling: SamplingReport,
    census: Vec<CensusRow>,
    corpora: Vec<Corpus>,
    rule_set_sha256: String,
) -> Report {
    let pdoc_complete = corpora
        .iter()
        .filter(|c| is_rate_bearing_pdoc(c))
        .all(Corpus::is_complete);
    let status = if pdoc_complete {
        RunStatus::Complete
    } else {
        RunStatus::InProgress
    };
    let overall_rate = if status == RunStatus::Complete {
        let measured: u64 = corpora
            .iter()
            .filter(|c| is_rate_bearing_pdoc(c))
            .map(|c| c.cases_measured)
            .sum();
        let matches: u64 = corpora
            .iter()
            .filter(|c| is_rate_bearing_pdoc(c))
            .map(|c| c.exact_matches)
            .sum();
        if measured > 0 {
            Some(format!("{matches}/{measured}"))
        } else {
            None
        }
    } else {
        None
    };
    let versions: Vec<String> = EMBEDDED_REGISTRY
        .versions()
        .into_iter()
        .map(|(v, _, _)| v.to_string())
        .collect();
    let pdoc_n = sampling.unique_form_boundary_july;
    let all_p_n = sampling.unique_form_boundary_july_all_legal_p;
    let grid_gap = corpora
        .iter()
        .find(|c| c.id == "grid-2026.1-pdoc-boundary")
        .map(|c| {
            let skipped = c.cases_defined.saturating_sub(c.cases_measured);
            format!(
                "Grid PDOC stratum: {pdoc_n} distinct capturable July forms (fingerprint-deduped). Queue finished: {} match, {} M-003 1¢ disagreements (counted), {skipped} PDOC step-2 would not advance (named, not in the rate). Superseded all-14-P count was {all_p_n}.",
                c.exact_matches,
                c.disagreements.len()
            )
        })
        .unwrap_or_else(|| {
            format!(
                "Grid PDOC stratum: {pdoc_n} distinct capturable July forms (fingerprint-deduped); capture not started. Superseded all-14-P count was {all_p_n}."
            )
        });
    Report {
        status,
        grid_version: sampling.grid_version.clone(),
        engine_build_sha256: ENGINE_BUILD_SHA256.to_string(),
        rule_set_versions: versions,
        rule_set_sha256,
        pdoc_identity: "2026-06-11".to_string(),
        browser: "chromium (cursor-ide-browser)".to_string(),
        harness_git_sha: "manual-capture".to_string(),
        latest_pdoc_retrieved_at: "2026-09-12T03:28:07Z".to_string(),
        sampling,
        census,
        corpora,
        agreement_rate: overall_rate,
        coverage_gaps: vec![
            "Quebec / QPIP: EngineError::JurisdictionNotSupported. Never a federal-only T2=0 success. docs/jurisdictions.md.".to_string(),
            "Interior log-ladder of grid 2026.1: engine invariants (§16.2 proptests over thirteen jurisdictions) and differential oracle only. Not a per-cell PDOC comparison.".to_string(),
            "Uncapturable boundary forms (P ∈ {1,2,4,2000}, $0 and sub-dollar gross): invariants + differential oracle — same as the interior ladder. Live PDOC salary UI cannot enter them.".to_string(),
            grid_gap,
            "January-only BC/NL/PE amounts retired from live PDOC: invariants + differential vs July sibling.".to_string(),
            "PDOC does not expose Option 2 (cumulative averaging). Live PDOC (observed 2026-06-11, screen WLCM) offers Salary, Commission, Pension, and CPP/EI verification only; there is no Option 2 / S1 / cumulative-averaging control, so no PDOC vectors were captured. Option 2 is a distinct oracle class `t4127_worked_examples` (T4127 Chapter 5 worked examples plus invariants) with weaker evidence than PDOC. It does not inherit the Option 1 PDOC agreement rate.".to_string(),
            "DateBeforeCoverage cells: engine error path, not a PDOC form.".to_string(),
            "M-003 PDOC midpoint direction: 164 one-cent disagreements counted in the rate (13 jurisdictions); 780 step-2 non-advances named out of the rate; condition unnamed; docs/findings/002-pdoc-midpoint-direction.md; ADR-003 rounding_compat pdoc is NotImplemented.".to_string(),
            "PDOC bonus class `pdoc-bonus-2026`: 20 defined cases. Expected amounts stay PENDING_PDOC until captured through tools/pdoc-oracle; never filled from the engine. Named pending; not in the overall Option 1 PDOC rate.".to_string(),
            "Year projection `year-projection-2026`: oracle class `invariants` (API tests 22–29). CPP/EI caps, YMPE/CPP2, BC January/July split, federal tax sum vs T1 within P cents, 53-week / 27-biweekly exemptions. Not a PDOC corpus. docs/year-projection.md.".to_string(),
        ],
        methodology: vec![
            "defect-vs-disagreement".to_string(),
            "M-001".to_string(),
            "M-002".to_string(),
            "M-003".to_string(),
            "ADR-003".to_string(),
        ],
    }
}

/// Render CONFORMANCE.md. Incomplete runs contain no overall agreement rate.
pub fn render_markdown(report: &Report) -> String {
    markdown::render(report, METHODOLOGY)
}

/// Canonical JSON (pretty, trailing newline).
pub fn render_json(report: &Report) -> Result<String, ReportError> {
    let mut out = serde_json::to_string_pretty(report)?;
    out.push('\n');
    Ok(out)
}

/// Write `CONFORMANCE.md` and `conformance.json` at the workspace root.
pub fn write_report(root: &Path, report: &Report) -> Result<(), ReportError> {
    fs::write(root.join("CONFORMANCE.md"), render_markdown(report))?;
    fs::write(root.join("conformance.json"), render_json(report)?)?;
    Ok(())
}

/// Regenerate and compare to the committed files. Non-zero if stale.
pub fn check_committed(root: &Path, report: &Report) -> Result<(), ReportError> {
    let md = render_markdown(report);
    let json = render_json(report)?;
    let got_md = fs::read_to_string(root.join("CONFORMANCE.md"))
        .map_err(|e| ReportError::Message(format!("CONFORMANCE.md missing: {e}")))?;
    let got_json = fs::read_to_string(root.join("conformance.json"))
        .map_err(|e| ReportError::Message(format!("conformance.json missing: {e}")))?;
    if got_md != md || got_json != json {
        return Err(ReportError::Message(
            "conformance report is stale; run `cargo run -p takehome-conformance -- --write`"
                .to_string(),
        ));
    }
    Ok(())
}

/// Exit 1 if any PDOC corpus is incomplete. The in-progress document is still valid.
pub fn require_complete(report: &Report) -> Result<(), ReportError> {
    if report.status != RunStatus::Complete {
        return Err(ReportError::Message(
            "run is in progress; refusing to treat it as a completed conformance report (no overall rate)"
                .to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VectorFile {
    vectors: Vec<PdocVector>,
}

#[derive(Debug, Deserialize)]
struct PdocVector {
    id: String,
    request: serde_json::Value,
    expected: ExpectedAmounts,
    oracle: OracleMeta,
}

#[derive(Debug, Deserialize)]
struct ExpectedAmounts {
    federal_tax: String,
    provincial_tax: String,
    cpp: String,
    cpp2: String,
    ei: String,
    total_deductions: String,
    net_pay: String,
}

impl ExpectedAmounts {
    fn is_pending(&self) -> bool {
        [
            self.federal_tax.as_str(),
            self.provincial_tax.as_str(),
            self.cpp.as_str(),
            self.cpp2.as_str(),
            self.ei.as_str(),
            self.total_deductions.as_str(),
            self.net_pay.as_str(),
        ]
        .contains(&"PENDING_PDOC")
    }

    fn map(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("federal_tax".into(), self.federal_tax.clone());
        m.insert("provincial_tax".into(), self.provincial_tax.clone());
        m.insert("cpp".into(), self.cpp.clone());
        m.insert("cpp2".into(), self.cpp2.clone());
        m.insert("ei".into(), self.ei.clone());
        m.insert("total_deductions".into(), self.total_deductions.clone());
        m.insert("net_pay".into(), self.net_pay.clone());
        m
    }
}

#[derive(Debug, Deserialize)]
struct OracleMeta {
    retrieved_at: String,
}

fn measure_m1() -> Result<Corpus, ReportError> {
    let file: VectorFile = serde_json::from_str(M1_VECTORS)?;
    let defined = u64::try_from(file.vectors.len())
        .map_err(|_| ReportError::Message("vector count".into()))?;
    let mut measured = 0u64;
    let mut matches = 0u64;
    let mut disagreements = Vec::new();
    for v in &file.vectors {
        if v.expected.is_pending() {
            continue;
        }
        measured += 1;
        let req = Request::from_json(&v.request.to_string())
            .map_err(|e| ReportError::Message(format!("{}: {e}", v.id)))?;
        let resp = calculate(&req, &EMBEDDED_REGISTRY)
            .map_err(|e| ReportError::Message(format!("{}: {e}", v.id)))?;
        let engine = employee_map(&resp.employee);
        let pdoc = v.expected.map();
        let delta = field_delta(&engine, &pdoc);
        if delta.is_empty() {
            matches += 1;
        } else {
            disagreements.push(Disagreement {
                id: v.id.clone(),
                request: v.request.clone(),
                engine,
                pdoc,
                delta,
                explanation: "engine and PDOC differ on identically specified inputs".to_string(),
                pdoc_believed_wrong: None,
            });
        }
        let _ = &v.oracle.retrieved_at;
    }
    Ok(Corpus::from_counts(
        "m1-ontario-2026-01",
        "pdoc",
        defined,
        measured,
        matches,
        disagreements,
    ))
}

fn measure_bonus() -> Result<Corpus, ReportError> {
    let file: VectorFile = serde_json::from_str(BONUS_VECTORS)?;
    let defined = u64::try_from(file.vectors.len())
        .map_err(|_| ReportError::Message("bonus vector count".into()))?;
    let mut measured = 0u64;
    let mut matches = 0u64;
    let mut disagreements = Vec::new();
    for v in &file.vectors {
        if v.expected.is_pending() {
            continue;
        }
        measured += 1;
        let req = Request::from_json(&v.request.to_string())
            .map_err(|e| ReportError::Message(format!("{}: {e}", v.id)))?;
        let resp = calculate(&req, &EMBEDDED_REGISTRY)
            .map_err(|e| ReportError::Message(format!("{}: {e}", v.id)))?;
        let engine = employee_map(&resp.employee);
        let pdoc = v.expected.map();
        let delta = field_delta(&engine, &pdoc);
        if delta.is_empty() {
            matches += 1;
        } else {
            disagreements.push(Disagreement {
                id: v.id.clone(),
                request: v.request.clone(),
                engine,
                pdoc,
                delta,
                explanation: "engine and PDOC differ on identically specified bonus inputs"
                    .to_string(),
                pdoc_believed_wrong: None,
            });
        }
        let _ = &v.oracle.retrieved_at;
    }
    Ok(Corpus::from_counts(
        "pdoc-bonus-2026",
        "pdoc",
        defined,
        measured,
        matches,
        disagreements,
    ))
}

/// Option 2 is not on PDOC. Lock the T4127 Chapter 5 worked examples (weaker
/// evidence than a PDOC corpus; never folded into the Option 1 agreement rate).
fn measure_option2() -> Result<Corpus, ReportError> {
    use takehome_core::formulas::option2::{
        annual_taxable_income_option2, period_tax_option2, scale_by_s1, Option2IncomeInputs, S1,
    };
    use takehome_core::PayPeriod;

    let mut matches = 0u64;
    let mut disagreements = Vec::new();
    let defined = 3u64;

    let s1_period2 = S1::from_pay_progress(
        PayPeriod::new(52).map_err(|e| ReportError::Message(e.to_string()))?,
        Some(1),
    )
    .map_err(|e| ReportError::Message(e.to_string()))?;
    if s1_period2.to_string() == "52/2" {
        matches += 1;
    } else {
        disagreements.push(Disagreement {
            id: "t4127-s1-weekly-period-2".to_string(),
            request: serde_json::json!({"s1": "52/2"}),
            engine: BTreeMap::from([("S1".into(), s1_period2.to_string())]),
            pdoc: BTreeMap::from([("S1".into(), "52/2".into())]),
            delta: BTreeMap::from([("S1".into(), s1_period2.to_string())]),
            explanation: "T4127 Table 5.1: weekly period 2 is the pair 52/2, not a float"
                .to_string(),
            pdoc_believed_wrong: None,
        });
    }

    let s1_26_21 = S1::new(26, 21).map_err(|e| ReportError::Message(e.to_string()))?;
    let projected = scale_by_s1(
        Money::parse("20500.00").map_err(|e| ReportError::Message(e.to_string()))?,
        s1_26_21,
    )
    .map_err(|e| ReportError::Message(e.to_string()))?;
    let a = annual_taxable_income_option2(&Option2IncomeInputs {
        s1: s1_26_21,
        gross_pay: Money::parse("20500.00").map_err(|e| ReportError::Message(e.to_string()))?,
        rpp: Money::ZERO,
        alimony_pre_1997: Money::ZERO,
        f5a: Money::ZERO,
        union_dues: Money::ZERO,
        prescribed_zone: Money::ZERO,
        annual_deductions: Money::ZERO,
        prior_non_periodic: Money::ZERO,
        prior_bonus_rrsp: Money::ZERO,
        f5b: Money::ZERO,
    })
    .map_err(|e| ReportError::Message(e.to_string()))?;
    if projected == Money::parse("25380.95").map_err(|e| ReportError::Message(e.to_string()))?
        && a == projected
    {
        matches += 1;
    } else {
        disagreements.push(Disagreement {
            id: "t4127-projected-income-26-21".to_string(),
            request: serde_json::json!({"I": "20500.00", "S1": "26/21"}),
            engine: BTreeMap::from([("A".into(), a.to_string())]),
            pdoc: BTreeMap::from([("A".into(), "25380.95".into())]),
            delta: BTreeMap::from([("A".into(), a.to_string())]),
            explanation: "T4127 Chapter 5: ($20,000+$500)×26/21 = $25,380.95".to_string(),
            pdoc_believed_wrong: None,
        });
    }

    let t = period_tax_option2(
        Money::parse("3560.17").map_err(|e| ReportError::Message(e.to_string()))?,
        Money::ZERO,
        s1_26_21,
        Money::parse("2736.40").map_err(|e| ReportError::Message(e.to_string()))?,
        Money::ZERO,
        Money::ZERO,
    )
    .map_err(|e| ReportError::Message(e.to_string()))?;
    if t == Money::parse("139.12").map_err(|e| ReportError::Message(e.to_string()))? {
        matches += 1;
    } else {
        disagreements.push(Disagreement {
            id: "t4127-period-tax-fictitious-example".to_string(),
            request: serde_json::json!({"T1_plus_T2": "3560.17", "S1": "26/21", "M": "2736.40"}),
            engine: BTreeMap::from([("T".into(), t.to_string())]),
            pdoc: BTreeMap::from([("T".into(), "139.12".into())]),
            delta: BTreeMap::from([("T".into(), t.to_string())]),
            explanation: "T4127 Chapter 5 fictitious tax example: $139.12 on the current $500"
                .to_string(),
            pdoc_believed_wrong: None,
        });
    }

    Ok(Corpus::from_counts(
        "t4127-option2-2026",
        "t4127_worked_examples",
        defined,
        defined,
        matches,
        disagreements,
    ))
}

fn measure_grid_pdoc(defined_from_grid: u64) -> Result<Corpus, ReportError> {
    let snap = grid_pdoc_snapshot()?;
    if snap.defined != defined_from_grid {
        return Err(ReportError::Message(format!(
            "grid PDOC snapshot defined {} != sampling unique July forms {defined_from_grid}",
            snap.defined
        )));
    }
    if snap.mismatched
        != u64::try_from(snap.disagreements.len())
            .map_err(|_| ReportError::Message("disagreement count".into()))?
    {
        return Err(ReportError::Message(
            "grid PDOC snapshot mismatched != disagreements.len()".into(),
        ));
    }
    Ok(Corpus::from_pdoc_queue(
        "grid-2026.1-pdoc-boundary",
        snap.defined,
        snap.matched,
        snap.mismatched,
        snap.skipped_step2,
        snap.disagreements,
    ))
}

fn employee_map(e: &takehome_core::EmployeeAmounts) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("federal_tax".into(), e.federal_tax.to_string());
    m.insert("provincial_tax".into(), e.provincial_tax.to_string());
    m.insert("cpp".into(), e.cpp.to_string());
    m.insert("cpp2".into(), e.cpp2.to_string());
    m.insert("ei".into(), e.ei.to_string());
    m.insert("total_deductions".into(), e.total_deductions.to_string());
    m.insert("net_pay".into(), e.net_pay.to_string());
    m
}

fn field_delta(
    engine: &BTreeMap<String, String>,
    pdoc: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut delta = BTreeMap::new();
    for (k, p) in pdoc {
        if let Some(e) = engine.get(k) {
            if e != p {
                if let (Ok(em), Ok(pm)) = (Money::parse(e), Money::parse(p)) {
                    let d = em.checked_sub(pm).unwrap_or(Money::ZERO);
                    delta.insert(k.clone(), d.to_string());
                } else {
                    delta.insert(k.clone(), format!("engine={e} pdoc={p}"));
                }
            }
        }
    }
    delta
}

fn hash_rules(dir: &Path) -> Result<String, ReportError> {
    let mut files = Vec::new();
    collect_json(dir, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for path in files {
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher.update(fs::read(&path)?);
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ReportError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{assemble_report, Corpus, Report, RunStatus};
    use std::collections::BTreeMap;
    use takehome_grid_gen::{CensusRow, OracleClass, SamplingReport};

    fn empty_sampling() -> SamplingReport {
        SamplingReport {
            grid_version: "2026.1".to_string(),
            grid_cases: 10,
            by_class: BTreeMap::new(),
            unresolvable_as_of: 0,
            resolvable: 10,
            boundary_resolvable: 4,
            unique_form_all_resolvable: 8,
            unique_form_log_resolvable: 4,
            unique_form_boundary_ignoring_as_of: 4,
            unique_form_boundary_with_rule_set: 4,
            unique_form_boundary_january: 4,
            unique_form_boundary_july_all_legal_p: 6,
            unique_form_boundary_july: 4,
            unique_form_boundary_july_uncapturable: 2,
            unique_form_boundary_july_claim1: 2,
            unique_form_boundary_july_claim1_common_p: 1,
            unique_form_boundary_july_by_province: BTreeMap::new(),
            hours_at_3s_full_grid: "0h 0m".to_string(),
            hours_at_3s_unique_form_all: "0h 0m".to_string(),
            hours_at_3s_unique_boundary_form: "0h 0m".to_string(),
            hours_at_3s_july_boundary: "0h 0m".to_string(),
            hours_at_3s_july_boundary_claim1: "0h 0m".to_string(),
            hours_at_3s_july_boundary_claim1_common_p: "0h 0m".to_string(),
            overnight_8h_capacity: 9600,
            cache_records_on_disk: 0,
            notes: vec![],
        }
    }

    fn census() -> Vec<CensusRow> {
        vec![CensusRow {
            jurisdiction: "ON".to_string(),
            pay_period: 52,
            rule_set_version: "2026-07-01".to_string(),
            oracle_class: OracleClass::Pdoc,
            cells: 4,
        }]
    }

    fn report(corpora: Vec<Corpus>) -> Report {
        assemble_report(empty_sampling(), census(), corpora, "deadbeef".to_string())
    }

    /// Test 43 — incomplete run: in_progress, no overall rate.
    #[test]
    fn incomplete_run_has_no_agreement_rate() {
        let m1 = Corpus::from_counts("m1", "pdoc", 20, 20, 20, Vec::new());
        let grid = Corpus::from_counts("grid-pdoc", "pdoc", 15264, 0, 0, Vec::new());
        let report = report(vec![m1, grid]);
        assert_eq!(report.status, RunStatus::InProgress);
        assert!(report.agreement_rate.is_none());
        let json = serde_json::to_value(&report).unwrap();
        assert!(
            json.get("agreement_rate").is_none(),
            "overall rate must be omitted, not zero: {json}"
        );
        assert!(report.corpora[0].agreement_rate.as_deref() == Some("20/20"));
        assert!(report.corpora[1].agreement_rate.is_none());
        let md = super::render_markdown(&report);
        assert!(md.contains("in progress") || md.contains("In progress"));
        assert!(
            !md.contains("Overall agreement rate"),
            "partial run must not publish an overall rate"
        );
        assert!(md.contains("M-001"));
        assert!(md.contains("M-002"));
        assert!(md.contains("disagreement"));
        assert!(md.contains("Quebec"));
        assert!(md.contains("interior"));
    }

    /// A finished PDOC run may publish a rate.
    #[test]
    fn complete_run_publishes_rate() {
        let m1 = Corpus::from_counts("m1", "pdoc", 20, 20, 20, Vec::new());
        let report = report(vec![m1]);
        assert_eq!(report.status, RunStatus::Complete);
        assert_eq!(report.agreement_rate.as_deref(), Some("20/20"));
        let md = super::render_markdown(&report);
        assert!(md.contains("Overall agreement rate"));
        assert!(md.contains("20/20"));
    }

    #[test]
    fn require_complete_rejects_partial() {
        let grid = Corpus::from_counts("grid-pdoc", "pdoc", 10, 1, 1, Vec::new());
        let report = report(vec![grid]);
        assert!(super::require_complete(&report).is_err());
    }

    fn dummy_disagreement(id: &str) -> super::Disagreement {
        super::Disagreement {
            id: id.to_string(),
            request: serde_json::json!({"province": "AB"}),
            engine: BTreeMap::from([("provincial_tax".into(), "1.00".into())]),
            pdoc: BTreeMap::from([("provincial_tax".into(), "0.99".into())]),
            delta: BTreeMap::from([("provincial_tax".into(), "0.01".into())]),
            explanation: "M-003".to_string(),
            pdoc_believed_wrong: None,
        }
    }

    /// Finished queue with step-2 skips: rate is compared forms only; M-003 counts.
    #[test]
    fn pdoc_queue_step2_skips_rate_on_compared_not_defined() {
        let grid = Corpus::from_pdoc_queue(
            "grid-pdoc",
            12,
            8,
            2,
            2,
            vec![dummy_disagreement("a"), dummy_disagreement("b")],
        );
        assert_eq!(grid.pending, 0);
        assert_eq!(grid.cases_defined, 12);
        assert_eq!(grid.cases_measured, 10);
        assert_eq!(grid.agreement_rate.as_deref(), Some("8/10"));
        assert!(grid.is_complete());
        let m1 = Corpus::from_counts("m1", "pdoc", 20, 20, 20, Vec::new());
        let report = report(vec![m1, grid]);
        assert_eq!(report.status, RunStatus::Complete);
        assert_eq!(report.agreement_rate.as_deref(), Some("28/30"));
        let md = super::render_markdown(&report);
        assert!(md.contains("Overall agreement rate"));
        assert!(md.contains("28/30"));
        assert!(md.contains("8/10"));
    }

    #[test]
    fn grid_pdoc_snapshot_matches_queue_length() {
        let snap = super::grid_pdoc_snapshot().unwrap();
        assert_eq!(snap.defined, 9762);
        assert_eq!(snap.matched, 8818);
        assert_eq!(snap.mismatched, 164);
        assert_eq!(snap.skipped_step2, 780);
        assert_eq!(snap.disagreements.len(), 164);
        assert_eq!(
            snap.matched + snap.mismatched + snap.skipped_step2,
            snap.defined
        );
    }

    fn money_cents(s: &str) -> i64 {
        let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
            (-1, r)
        } else {
            (1, s)
        };
        let (d, c) = rest.split_once('.').expect("money has a dot");
        assert_eq!(c.len(), 2);
        sign * (d.parse::<i64>().unwrap() * 100 + c.parse::<i64>().unwrap())
    }

    /// Exact half-cent of annual/P: 2*annual_cents / P is an odd integer.
    fn period_is_half_cent(annual_cents: i64, p: u16) -> bool {
        let p = i64::from(p);
        p != 0
            && (annual_cents * 2).rem_euclid(p) == 0
            && ((annual_cents * 2) / p).rem_euclid(2) == 1
    }

    /// Corpus census: every delta is ±1¢; T2/P half-cent vs not, by field class.
    #[test]
    fn m003_corpus_every_delta_is_one_cent_and_t2_halfcent_rate() {
        use takehome_core::rules::loader::EMBEDDED_REGISTRY;
        use takehome_core::{calculate, Request};
        let snap = super::grid_pdoc_snapshot().unwrap();
        let mut t2_half = 0u32;
        let mut t2_not = 0u32;
        let mut t1_half = 0u32;
        let mut prov_n = 0u32;
        let mut fed_n = 0u32;
        let mut cpp_n = 0u32;
        for d in &snap.disagreements {
            for v in d.delta.values() {
                let n = money_cents(v).unsigned_abs();
                assert_eq!(n, 1, "{} delta not 1¢: {v}", d.id);
            }
            let req = Request::from_json(&d.request.to_string()).unwrap();
            let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            let p = req.pay_period.get();
            let t2 = money_cents(&resp.breakdown.t2.to_string());
            let t1 = money_cents(&resp.breakdown.t1.to_string());
            if d.delta.contains_key("provincial_tax") {
                prov_n += 1;
                if period_is_half_cent(t2, p) {
                    t2_half += 1;
                } else {
                    t2_not += 1;
                }
            }
            if d.delta.contains_key("federal_tax") {
                fed_n += 1;
                if period_is_half_cent(t1, p) {
                    t1_half += 1;
                }
            }
            if d.delta.contains_key("cpp") {
                cpp_n += 1;
            }
        }
        assert_eq!(prov_n, 65);
        assert_eq!(cpp_n, 82);
        assert_eq!(fed_n, 19);
        assert_eq!(t2_half, 34);
        assert_eq!(t2_not, 31);
        assert_eq!(t1_half, 9);
    }

    /// Test 52 — named pending PDOC bonus class does not wipe the Option 1 rate.
    #[test]
    fn bonus_pending_does_not_block_overall_pdoc_rate() {
        let m1 = Corpus::from_counts("m1-ontario-2026-01", "pdoc", 20, 20, 20, Vec::new());
        let bonus = Corpus::from_counts("pdoc-bonus-2026", "pdoc", 20, 0, 0, Vec::new());
        let grid = Corpus::from_pdoc_queue(
            "grid-2026.1-pdoc-boundary",
            12,
            8,
            2,
            2,
            vec![dummy_disagreement("a"), dummy_disagreement("b")],
        );
        let year = Corpus::from_counts("year-projection-2026", "invariants", 8, 8, 8, Vec::new());
        let option2 = Corpus::from_counts(
            "t4127-option2-2026",
            "t4127_worked_examples",
            3,
            3,
            3,
            Vec::new(),
        );
        assert_eq!(bonus.pending, 20);
        assert!(!bonus.is_complete());
        let report = report(vec![m1, bonus, year, option2, grid]);
        assert_eq!(report.status, RunStatus::Complete);
        assert_eq!(report.agreement_rate.as_deref(), Some("28/30"));
        let md = super::render_markdown(&report);
        assert!(md.contains("Overall agreement rate"));
        assert!(md.contains("28/30"));
        assert!(md.contains("pdoc-bonus-2026"));
        assert!(md.contains("year-projection-2026"));
        assert!(md.contains("t4127-option2-2026"));
        assert!(
            md.contains("Does not inherit the Option 1 PDOC agreement rate")
                || md.contains("does not inherit the Option 1 PDOC agreement rate")
        );
        assert!(md.contains("PENDING_PDOC"));
    }

    /// Test 43 stays: an unstarted *rate-bearing* PDOC corpus still forbids a rate.
    #[test]
    fn unfinished_grid_pdoc_still_blocks_overall_rate() {
        let m1 = Corpus::from_counts("m1", "pdoc", 20, 20, 20, Vec::new());
        let bonus = Corpus::from_counts("pdoc-bonus-2026", "pdoc", 20, 0, 0, Vec::new());
        let grid = Corpus::from_counts("grid-pdoc", "pdoc", 15264, 0, 0, Vec::new());
        let report = report(vec![m1, bonus, grid]);
        assert_eq!(report.status, RunStatus::InProgress);
        assert!(report.agreement_rate.is_none());
    }

    /// Test 52 — year-projection corpus: eight API invariants, oracle `invariants`.
    #[test]
    fn year_projection_corpus_is_invariants_oracle() {
        let corpus = super::year::measure().expect("year projection");
        assert_eq!(corpus.id, "year-projection-2026");
        assert_eq!(corpus.oracle_class, "invariants");
        assert_eq!(corpus.cases_defined, 8);
        assert_eq!(corpus.cases_measured, 8);
        assert_eq!(corpus.pending, 0);
        assert_eq!(corpus.exact_matches, 8);
        assert!(
            corpus.disagreements.is_empty(),
            "{:?}",
            corpus.disagreements
        );
        assert!(corpus.agreement_rate.is_none());
    }

    /// Test 52 — Option 2 stays a weaker oracle and is not folded into PDOC.
    #[test]
    fn option2_corpus_is_t4127_worked_examples_not_pdoc() {
        let corpus = super::measure_option2().expect("option 2");
        assert_eq!(corpus.id, "t4127-option2-2026");
        assert_eq!(corpus.oracle_class, "t4127_worked_examples");
        assert_eq!(corpus.cases_defined, 3);
        assert_eq!(corpus.exact_matches, 3);
        assert!(corpus.agreement_rate.is_none());
    }

    /// Test 52/53 — M4 engine vs committed PDOC queue: 8838/9002, all 164 listed.
    #[test]
    fn m4_repo_report_pdoc_rate_has_not_regressed() {
        let report = super::report_from_repo().expect("report");
        assert_eq!(report.status, RunStatus::Complete);
        assert_eq!(report.agreement_rate.as_deref(), Some("8838/9002"));
        let ids: Vec<_> = report.corpora.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"pdoc-bonus-2026"));
        assert!(ids.contains(&"year-projection-2026"));
        assert!(ids.contains(&"t4127-option2-2026"));
        let bonus = report
            .corpora
            .iter()
            .find(|c| c.id == "pdoc-bonus-2026")
            .unwrap();
        assert_eq!(bonus.pending, 20);
        assert_eq!(bonus.cases_measured, 0);
        let grid = report
            .corpora
            .iter()
            .find(|c| c.id == "grid-2026.1-pdoc-boundary")
            .unwrap();
        assert_eq!(grid.disagreements.len(), 164);
        assert_eq!(grid.exact_matches, 8818);
        let md = super::render_markdown(&report);
        assert!(md.contains("**Overall agreement rate (PDOC corpora): `8838/9002`.**"));
        let snap = super::grid_pdoc_snapshot().unwrap();
        for d in &snap.disagreements {
            assert!(
                md.contains(&format!("`{}`", d.id)),
                "CONFORMANCE.md must list disagreement {}",
                d.id
            );
        }
        assert!(md.contains("AB-P10-11704.49-F0-P0"));
        assert!(md.contains("YT-P24-208.33-F0-P0"));
        assert!(md.contains("t4127-option2-2026"));
        assert!(md.contains("year-projection-2026"));
        assert!(md.contains("pdoc-bonus-2026"));
        assert!(md.contains("PENDING_PDOC"));
        assert!(md.contains("PDOC does not expose Option 2"));
        assert!(report.rule_set_versions.iter().any(|v| v == "2027-01-01"));
        assert_eq!(report.sampling.unique_form_boundary_july, 9762);
    }

    /// Test 54 — integer-cent probe: a T1/T2 half-cent-down arm cannot zero the 164.
    #[test]
    fn half_cent_down_arm_cannot_zero_the_164() {
        use takehome_core::request::{Request, RequestError, RoundingCompat};
        use takehome_core::rules::loader::EMBEDDED_REGISTRY;
        use takehome_core::{calculate, EngineError};

        let snap = super::grid_pdoc_snapshot().unwrap();
        let mut t2_half_engine_higher = 0u32;
        let mut t2_half_engine_lower = 0u32;
        let mut t2_not_half = 0u32;
        let mut cpp_only = 0u32;
        let mut mixed_direction_tax = 0u32;
        for d in &snap.disagreements {
            let req = Request::from_json(&d.request.to_string()).unwrap();
            let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            let p = req.pay_period.get();
            let t2 = money_cents(&resp.breakdown.t2.to_string());
            let tax_keys: Vec<_> = ["cpp", "federal_tax", "provincial_tax"]
                .into_iter()
                .filter(|k| d.delta.contains_key(*k))
                .collect();
            if tax_keys == ["cpp"] {
                cpp_only += 1;
            }
            if let Some(delta) = d.delta.get("provincial_tax") {
                let sign = money_cents(delta);
                if sign < 0 {
                    mixed_direction_tax += 1;
                }
                if period_is_half_cent(t2, p) {
                    if sign > 0 {
                        t2_half_engine_higher += 1;
                    } else {
                        t2_half_engine_lower += 1;
                    }
                } else {
                    t2_not_half += 1;
                }
            }
        }
        assert_eq!(cpp_only, 81);
        assert_eq!(t2_not_half, 31);
        assert!(t2_half_engine_higher > 0);
        assert_eq!(
            t2_half_engine_lower, 0,
            "T2/P half-cent provincials are uniformly engine-higher; mixed direction lives off-half"
        );
        assert!(mixed_direction_tax > 0);
        let remaining = 164u32.saturating_sub(t2_half_engine_higher);
        assert!(
            remaining > 0,
            "a T2/P half-cent-down arm cannot be a discriminator for the 164"
        );

        let mut req = Request::from_json(&snap.disagreements[0].request.to_string()).unwrap();
        req.rounding_compat = RoundingCompat::Pdoc;
        let err = calculate(&req, &EMBEDDED_REGISTRY).expect_err("pdoc arm stays typed");
        assert!(matches!(
            err,
            EngineError::Request(RequestError::RoundingCompatPdocNotImplemented)
        ));
    }
}
