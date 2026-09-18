//! takehome-core: no IO, no network, no clock, no environment access.
//! Given a Request and a RuleSet, returns a Response. Deterministic, forever.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

pub mod decimal;
pub mod formulas;
pub mod jurisdictions;
pub mod request;
pub mod response;
pub mod rounding;
pub mod rules;

pub use decimal::{DecimalError, Money, Rate, Ratio};
pub use formulas::option2::S1;
pub use jurisdictions::{
    jurisdictions_json, list_jurisdictions, JurisdictionListing, JurisdictionStatus,
    QUEBEC_UNSUPPORTED_REASON,
};
pub use request::{
    BonusMethod, ClaimCode, K2Method, PayPeriod, Province, Request, RequestError, RoundingCompat,
};
pub use response::{
    AnnualProjection, Breakdown, Citation, Count, EmployeeAmounts, EmployerAmounts, Response,
    Warning, ENGINE_BUILD_SHA256,
};
pub use rules::diff::{diff_rule_sets, diff_rule_sets_json, RuleSetDiff};
pub use rules::loader::{RuleSetListing, RuleSetVersionStatus};
pub use rules::registry::{Registry, RuleError};

use serde::Serialize;
use thiserror::Error;

#[cfg(test)]
mod jurisdiction_invariants;

/// Fatal engine failure. Non-fatal notes go in [`Response::warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error(transparent)]
    Rule(#[from] RuleError),
    #[error(transparent)]
    Request(#[from] RequestError),
    #[error(transparent)]
    Decimal(#[from] DecimalError),
    /// A named jurisdiction the engine will not calculate. Quebec must never
    /// look like a successful federal-only run with T2 = 0.
    #[error("jurisdiction {jurisdiction} is not supported. {reason}")]
    JurisdictionNotSupported {
        jurisdiction: String,
        reason: String,
    },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WireError {
    error: WireErrorBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct WireErrorBody {
    code: String,
    message: String,
}

impl EngineError {
    /// Stable error code on the JSON wire (spec §9.3).
    pub fn wire_code(&self) -> &'static str {
        match self {
            Self::Request(RequestError::Serde { .. }) => "malformed_json",
            Self::Request(_) | Self::Decimal(_) => "invalid_request",
            Self::Rule(_) => "rule",
            Self::JurisdictionNotSupported { .. } => "jurisdiction_not_supported",
            Self::Message(_) => "engine",
        }
    }

    /// `{"error":{"code","message"}}`. Never a success [`Response`].
    pub fn to_wire_json(&self) -> String {
        let payload = WireError {
            error: WireErrorBody {
                code: self.wire_code().to_string(),
                message: self.to_string(),
            },
        };
        match serde_json::to_string(&payload) {
            Ok(body) => body,
            Err(_) => {
                "{\"error\":{\"code\":\"engine\",\"message\":\"error serialization failed\"}}"
                    .to_string()
            }
        }
    }
}

fn engine_message(error: impl std::fmt::Display) -> EngineError {
    EngineError::Message(error.to_string())
}

/// JSON request in; JSON [`Response`] or error object out (spec §5.4 / §9.3).
///
/// WASM, Python, and CLI bindings call this so every runtime shares one wire
/// contract. Never panics: malformed input is an error object.
pub fn calculate_wire(request_json: &str) -> String {
    match calculate_wire_result(request_json) {
        Ok(body) => body,
        Err(error) => error.to_wire_json(),
    }
}

fn calculate_wire_result(request_json: &str) -> Result<String, EngineError> {
    let req = Request::from_json(request_json)?;
    let resp = calculate(&req, &crate::rules::loader::EMBEDDED_REGISTRY)?;
    serde_json::to_string(&resp).map_err(|e| EngineError::Message(e.to_string()))
}

/// Embedded T4127 rule-set editions in coverage order (spec §9.1).
pub fn list_rule_set_versions() -> RuleSetListing {
    crate::rules::loader::list_embedded_rule_set_versions()
}

/// Pretty JSON for `listRuleSetVersions` and `GET /v1/rule-set-versions`.
pub fn rule_set_versions_json() -> String {
    match serde_json::to_string_pretty(&list_rule_set_versions()) {
        Ok(body) => body,
        Err(_) => {
            "{\"error\":{\"code\":\"engine\",\"message\":\"rule set listing failed to serialize\"}}"
                .to_string()
        }
    }
}

/// Calculate payroll deductions from the rule set effective on `req.as_of`.
///
/// Option 1 is T4127 Chapter 4 (`A = P × (I − …)`, `T = (T1+T2)/P + L`).
/// Option 2 is T4127 Chapter 5 cumulative averaging (`S1`, `A` with YTD, `T` with M/M1).
///
/// A proposed (not yet enacted) rule set is returned with a `RULE_SET_PROPOSED`
/// warning that names the announcement date (spec open question 5).
///
/// `rounding_compat: pdoc` is a typed [`RequestError::RoundingCompatPdocNotImplemented`]
/// until finding 002 names PDOC’s midpoint condition (ADR-003).
pub fn calculate(req: &Request, registry: &Registry) -> Result<Response, EngineError> {
    use crate::formulas::annual_income::{
        annual_taxable_income, f5, f5a, f5b, AnnualTaxableIncomeInputs,
    };
    use crate::formulas::bpa::resolve_basic_personal_amount;
    use crate::formulas::cpp::{cpp2_contribution, cpp_contribution};
    use crate::formulas::credits::{k1, k1p, k2, k4, k4p, GrossEmploymentIncome};
    use crate::formulas::ei::{ei_premium, employer_ei_premium, qpip_premium};
    use crate::formulas::federal_tax::{federal_t1, federal_t3};
    use crate::formulas::option2::{
        annual_taxable_income_option2, k2_option2, option2_bonus_groups, period_tax_option2,
        scale_by_s1, Option2IncomeInputs, S1,
    };
    use crate::formulas::province::alberta::alberta_k5p;
    use crate::formulas::province::ontario::{ontario_t4, select_ontario_bracket};
    use crate::rounding::{round_tax_to_cent, Granularity};
    use crate::rules::schema::JurisdictionCode;

    if req.federal_claim_code.is_some() && req.federal_tc.is_some() {
        return Err(RequestError::AmbiguousFederalClaim.into());
    }
    if req.provincial_claim_code.is_some() && req.provincial_tcp.is_some() {
        return Err(RequestError::AmbiguousProvincialClaim.into());
    }
    if req.province == Province::Qc {
        return Err(EngineError::JurisdictionNotSupported {
            jurisdiction: "QC".to_string(),
            reason: QUEBEC_UNSUPPORTED_REASON.to_string(),
        });
    }
    if req.rounding_compat == crate::request::RoundingCompat::Pdoc {
        return Err(RequestError::RoundingCompatPdocNotImplemented.into());
    }

    let set = registry.resolve(req.as_of)?;
    let fed = set
        .jurisdictions
        .get(&JurisdictionCode("FED".to_string()))
        .ok_or_else(|| EngineError::Message("rule set has no FED jurisdiction".to_string()))?;
    let province_code = JurisdictionCode(req.province.as_str().to_string());
    let provincial = match req.province {
        Province::OutsideCanada => None,
        _ => Some(set.jurisdictions.get(&province_code).ok_or_else(|| {
            EngineError::Message(format!(
                "rule set has no {} jurisdiction",
                req.province.as_str()
            ))
        })?),
    };
    let option = req.calculation_option;

    let zero = Money::ZERO;
    let one = Money::parse("1")?;
    let p_money = Money::parse(&req.pay_period.get().to_string())?;
    let ytd_cpp = req.ytd_cpp.unwrap_or(zero);
    let ytd_cpp2 = req.ytd_cpp2.unwrap_or(zero);
    let ytd_ei = req.ytd_ei.unwrap_or(zero);
    let cpp_exempt = req.cpp_exempt.unwrap_or(false);
    let non_periodic = req.non_periodic_pay()?;
    let pi = req.pensionable_earnings();
    let ie = req.insurable_earnings();
    let i_for_a = req.periodic_income_for_a();
    let c = cpp_contribution(
        pi,
        ytd_cpp,
        req.pay_period,
        req.cpp_months,
        &set.cpp,
        cpp_exempt,
    )?;
    let c2 = cpp2_contribution(
        pi,
        req.ytd_pensionable_earnings.unwrap_or(zero),
        ytd_cpp2,
        req.cpp_months,
        &set.cpp,
        cpp_exempt,
    )?;
    let ei = ei_premium(ie, ytd_ei, &set.ei, false)?;
    let qpip = qpip_premium(
        ie,
        req.ytd_qpip.unwrap_or(zero),
        req.province,
        &set.qpip,
        false,
    )?;
    let employer_ei = employer_ei_premium(ie, zero, &set.ei, false)?;

    let f5_value =
        f5(c, c2, set.cpp.first_additional_rate, set.cpp.total_rate).map_err(engine_message)?;
    let f5a_value = if pi.is_zero() {
        zero
    } else {
        f5a(f5_value, pi, non_periodic).map_err(engine_message)?
    };
    let f5b_value = if pi.is_zero() {
        zero
    } else {
        f5b(f5_value, pi, non_periodic).map_err(engine_message)?
    };
    let periods_remaining = req
        .pay_periods_elapsed
        .map(|elapsed| req.pay_period.get().saturating_sub(elapsed).max(1))
        .unwrap_or(req.pay_period.get());
    let p_count = Count::new(req.pay_period.get());
    let pr_count = Count::new(periods_remaining);
    let pm_count = Count::new(u16::from(req.cpp_months));
    let option2 = option == crate::rules::schema::CalculationOption::Option2;
    let s1 = if option2 {
        S1::from_pay_progress(req.pay_period, req.pay_periods_elapsed).map_err(engine_message)?
    } else {
        S1::for_option1(req.pay_period).map_err(engine_message)?
    };
    let m_federal = req.ytd_federal_tax.unwrap_or(zero);
    let m_provincial = req.ytd_provincial_tax.unwrap_or(zero);
    let m = m_federal.checked_add(m_provincial)?;
    let m1 = zero;
    let union_dues = req.union_dues.unwrap_or(zero);
    let rpp = req.rpp.unwrap_or(zero);
    let annual_deductions = req
        .child_care_expenses
        .unwrap_or(zero)
        .checked_add(req.estimated_annual_expenses.unwrap_or(zero))?;
    let prescribed_zone = req.prescribed_zone_deduction.unwrap_or(zero);
    let alimony = req.alimony.unwrap_or(zero);
    let i_projected = if option2 {
        i_for_a.checked_add(req.ytd_income.unwrap_or(zero))?
    } else {
        i_for_a
    };
    let rpp_projected = if option2 {
        rpp.checked_add(req.ytd_rpp.unwrap_or(zero))?
    } else {
        rpp
    };
    let f5a_projected = if option2 {
        f5a_value.checked_add(req.f5a_ytd.unwrap_or(zero))?
    } else {
        f5a_value
    };
    let union_projected = if option2 {
        union_dues.checked_add(req.ytd_union_dues.unwrap_or(zero))?
    } else {
        union_dues
    };
    let option2_inputs = Option2IncomeInputs {
        s1,
        gross_pay: i_projected,
        rpp: rpp_projected,
        alimony_pre_1997: alimony,
        f5a: f5a_projected,
        union_dues: union_projected,
        prescribed_zone,
        annual_deductions,
        prior_non_periodic: req.ytd_bonus.unwrap_or(zero),
        prior_bonus_rrsp: req.ytd_bonus_rrsp.unwrap_or(zero),
        f5b: if option2 {
            f5b_value.checked_add(req.f5b_ytd.unwrap_or(zero))?
        } else {
            req.f5b_ytd.unwrap_or(zero)
        },
    };
    let bonus_groups = if non_periodic.is_zero() {
        None
    } else if option2 {
        Some(
            option2_bonus_groups(
                &Option2IncomeInputs {
                    f5b: req.f5b_ytd.unwrap_or(zero),
                    ..option2_inputs
                },
                non_periodic,
                req.bonus_rrsp.unwrap_or(zero),
                f5b_value,
            )
            .map_err(engine_message)?,
        )
    } else {
        Some(
            formulas::bonus::bonus_income_groups(
                req.bonus_method,
                &formulas::bonus::BonusIncomeInputs {
                    pay_period: req.pay_period,
                    periods_remaining,
                    periodic_income: i_for_a,
                    rpp,
                    alimony_pre_1997: alimony,
                    f5a: f5a_value,
                    union_dues,
                    prescribed_zone,
                    annual_deductions,
                    current_non_periodic: non_periodic,
                    bonus_rrsp: req.bonus_rrsp.unwrap_or(zero),
                    f5b: f5b_value,
                    prior_non_periodic: req.ytd_bonus.unwrap_or(zero),
                    prior_bonus_rrsp: req.ytd_bonus_rrsp.unwrap_or(zero),
                    f5b_ytd: req.f5b_ytd.unwrap_or(zero),
                    ytd_periodic_income: req.ytd_income.unwrap_or(zero),
                    ytd_rpp: req.ytd_rpp.unwrap_or(zero),
                    ytd_f5a: req.f5a_ytd.unwrap_or(zero),
                    ytd_union_dues: req.ytd_union_dues.unwrap_or(zero),
                },
            )
            .map_err(engine_message)?,
        )
    };
    let a_without = bonus_groups
        .as_ref()
        .map(|g| g.a_without())
        .transpose()
        .map_err(engine_message)?;
    let a = if let Some(groups) = bonus_groups.as_ref() {
        groups.a_with().map_err(engine_message)?
    } else if option2 {
        annual_taxable_income_option2(&option2_inputs).map_err(engine_message)?
    } else {
        annual_taxable_income(&AnnualTaxableIncomeInputs {
            pay_period: req.pay_period,
            gross_pay: i_for_a,
            rpp,
            alimony_pre_1997: alimony,
            f5a: f5a_value,
            union_dues,
            prescribed_zone,
            annual_deductions,
        })
        .map_err(engine_message)?
    };

    let fed_bpa_definition = fed.basic_personal_amount.get(option);
    let bpaf =
        resolve_basic_personal_amount(a, fed_bpa_definition, None).map_err(engine_message)?;
    let provincial_bpa = match provincial {
        Some(jurisdiction) => resolve_basic_personal_amount(
            a,
            jurisdiction.basic_personal_amount.get(option),
            Some(fed_bpa_definition),
        )
        .map_err(engine_message)?,
        None => zero,
    };
    let resolve_claim = |claim: Option<ClaimCode>, direct: Option<Money>, default: Money| {
        if let Some(value) = direct {
            value
        } else {
            match claim {
                Some(ClaimCode::Code(0)) | Some(ClaimCode::E) => zero,
                Some(ClaimCode::Code(_)) | None => default,
            }
        }
    };
    let tc = resolve_claim(req.federal_claim_code, req.federal_tc, bpaf);
    let tcp = resolve_claim(
        req.provincial_claim_code,
        req.provincial_tcp,
        provincial_bpa,
    );
    let cea = fed.canada_employment_amount.unwrap_or(zero);

    let annualize = |amount: Money, maximum: Money| -> Result<Money, EngineError> {
        Ok(amount.checked_mul(p_money)?.min(maximum))
    };
    let annual_cpp = annualize(c, set.cpp.total_max)?;
    let annual_cpp2 = annualize(c2, set.cpp.second_additional_max)?;
    let annual_ei = annualize(ei, set.ei.employee_max)?;
    let annual_qpip = annualize(qpip, set.qpip.employee_max)?;
    let additional_tax = req.additional_tax_requested.unwrap_or(zero);

    let source_document = set.source_document.clone();
    let source_url = set.source_url.clone();
    let citations = ["R", "K", "V", "KP", "CEA", "BPAF", "TC", "TCP", "TB"]
        .into_iter()
        .map(|factor| Citation {
            factor: factor.to_string(),
            source_document: source_document.clone(),
            source_url: source_url.clone(),
        })
        .collect::<Vec<_>>();

    if a.is_negative() {
        let total_deductions = c
            .checked_add(c2)?
            .checked_add(ei)?
            .checked_add(qpip)?
            .checked_add(additional_tax)?
            .checked_add(union_dues)?;
        let mut breakdown = Breakdown::zeros();
        breakdown.a = a;
        breakdown.t = additional_tax;
        breakdown.c = c;
        breakdown.c2 = c2;
        breakdown.ei = ei;
        breakdown.f5 = f5_value;
        breakdown.bpaf = bpaf;
        breakdown.f5a = f5a_value;
        breakdown.f5b = f5b_value;
        breakdown.d = ytd_cpp;
        breakdown.d1 = ytd_ei;
        breakdown.d2 = ytd_cpp2;
        breakdown.p = p_count;
        breakdown.pr = pr_count;
        breakdown.pm = pm_count;
        breakdown.s1 = s1;
        breakdown.m = m;
        breakdown.m1 = m1;
        breakdown.cea = cea;
        breakdown.tc = tc;
        breakdown.tcp = tcp;
        breakdown.ie = ie;
        breakdown.qpip = qpip;
        return Ok(Response {
            rule_set_version: set.rule_set_version.clone(),
            engine_version: Response::engine_version().to_string(),
            engine_build_sha256: Response::engine_build_sha256().to_string(),
            prorated_rules_applied: false,
            employee: EmployeeAmounts {
                federal_tax: additional_tax,
                provincial_tax: zero,
                total_tax: additional_tax,
                cpp: c,
                cpp2: c2,
                ei,
                qpip,
                total_deductions,
                net_pay: req
                    .cheque_gross()?
                    .checked_sub(total_deductions)?
                    .floor_at_zero(),
            },
            employer: EmployerAmounts {
                cpp: c,
                cpp2: c2,
                ei: employer_ei,
                qpip: zero,
            },
            annual_projection: AnnualProjection {
                taxable_income: a,
                federal_tax: zero,
                provincial_tax: zero,
                cpp: annual_cpp,
                cpp2: annual_cpp2,
                ei: annual_ei,
                qpip: annual_qpip,
            },
            breakdown,
            citations,
            warnings: vec![],
        });
    }

    let fed_brackets = fed.brackets.get(option);
    let fed_bracket =
        formulas::federal_tax::select_federal_bracket(a, fed_brackets).map_err(engine_message)?;
    let r = fed_bracket.rate;
    let k = fed_bracket.constant;
    let federal_lowest = *fed.lowest_rate.get(option);
    let k1_value = k1(federal_lowest, tc).map_err(engine_message)?;
    let c_regular = cpp_contribution(
        i_for_a,
        ytd_cpp,
        req.pay_period,
        req.cpp_months,
        &set.cpp,
        cpp_exempt,
    )?;
    let c_current = formulas::bonus::contribution_on_amount(non_periodic, set.cpp.total_rate)
        .map_err(engine_message)?;
    let c_prior =
        formulas::bonus::contribution_on_amount(req.ytd_bonus.unwrap_or(zero), set.cpp.total_rate)
            .map_err(engine_message)?;
    let ei_regular = ei_premium(i_for_a, ytd_ei, &set.ei, false)?;
    let ei_current = formulas::bonus::contribution_on_amount(non_periodic, set.ei.employee_rate)
        .map_err(engine_message)?;
    let ei_prior = formulas::bonus::contribution_on_amount(
        req.ytd_bonus.unwrap_or(zero),
        set.ei.employee_rate,
    )
    .map_err(engine_message)?;
    let k2_from_parts = |lowest: Rate, include_current: bool| {
        formulas::bonus::k2_non_periodic(
            lowest,
            req.pay_period,
            c_regular,
            c_current,
            c_prior,
            ei_regular,
            ei_current,
            ei_prior,
            include_current,
            &set.cpp,
            &set.ei,
        )
        .map_err(engine_message)
    };
    let pe_k2 = req
        .pensionable_earnings
        .unwrap_or(i_for_a)
        .checked_add(req.ytd_pensionable_earnings.unwrap_or(zero))?;
    let ie_k2 = req
        .insurable_earnings
        .unwrap_or(i_for_a)
        .checked_add(req.ytd_insurable_earnings.unwrap_or(zero))?;
    let k2_option2_at = |lowest: Rate| {
        k2_option2(
            lowest,
            s1,
            pe_k2,
            ie_k2,
            req.ytd_bonus.unwrap_or(zero),
            cpp_exempt,
            &set.cpp,
            &set.ei,
        )
        .map_err(engine_message)
    };
    let k2_value = if option2 {
        k2_option2_at(federal_lowest)?
    } else if non_periodic.is_zero() {
        k2(
            federal_lowest,
            req.pay_period,
            c,
            ytd_cpp,
            ei,
            ytd_ei,
            periods_remaining,
            req.cpp_months,
            req.k2_method,
            &set.cpp,
            &set.ei,
        )
        .map_err(engine_message)?
    } else {
        k2_from_parts(federal_lowest, true)?
    };
    let annual_gross = if option2 {
        let projected = scale_by_s1(i_projected, s1).map_err(engine_message)?;
        projected
            .checked_add(req.ytd_bonus.unwrap_or(zero))?
            .checked_add(non_periodic)?
    } else if non_periodic.is_zero() {
        i_for_a.checked_mul(p_money)?
    } else {
        i_for_a
            .checked_mul(p_money)?
            .checked_add(non_periodic)?
            .checked_add(req.ytd_bonus.unwrap_or(zero))?
    };
    let k4_value = k4(
        federal_lowest,
        GrossEmploymentIncome::new(annual_gross),
        cea,
    )
    .map_err(engine_message)?;
    let federal_exempt = matches!(req.federal_claim_code, Some(ClaimCode::E));
    let t3 = if federal_exempt {
        zero
    } else {
        federal_t3(a, k1_value, k2_value, zero, k4_value, fed_brackets).map_err(engine_message)?
    };
    let lcf_params = fed
        .lcf
        .as_ref()
        .ok_or_else(|| EngineError::Message("federal LCF rules missing".to_string()))?;
    let lcf = match req.lcf_purchase {
        Some(purchase) => round_tax_to_cent(
            purchase
                .floor_at_zero()
                .checked_mul_rate(lcf_params.rate)?
                .min(lcf_params.max)
                .checked_div(one)?,
        ),
        None => zero,
    };
    let t1 = if federal_exempt {
        zero
    } else {
        federal_t1(
            t3,
            req.pay_period,
            req.lcf_purchase,
            lcf_params,
            req.province,
            fed.abatement,
            fed.surtax_flat,
        )
        .map_err(engine_message)?
    };

    let provincial_exempt = matches!(req.provincial_claim_code, Some(ClaimCode::E));
    let (
        v,
        kp,
        k1p_value,
        k2p_value,
        t4,
        v1,
        v2,
        y,
        s,
        t2,
        lcp,
        k4p_value,
        k5p_value,
        provincial_prorated,
    ) = if let Some(provincial) = provincial {
        let provincial_brackets = provincial.brackets.get(option);
        let provincial_bracket =
            select_ontario_bracket(a, provincial_brackets).map_err(engine_message)?;
        let v = provincial_bracket.rate;
        let kp = provincial_bracket.constant;
        let k1p_value = k1p(*provincial.lowest_rate.get(option), tcp).map_err(engine_message)?;
        let k2p_value = if option2 {
            k2_option2_at(*provincial.lowest_rate.get(option))?
        } else if non_periodic.is_zero() {
            k2(
                *provincial.lowest_rate.get(option),
                req.pay_period,
                c,
                ytd_cpp,
                ei,
                ytd_ei,
                periods_remaining,
                req.cpp_months,
                req.k2_method,
                &set.cpp,
                &set.ei,
            )
            .map_err(engine_message)?
        } else {
            k2_from_parts(*provincial.lowest_rate.get(option), true)?
        };
        let k5p_value = if provincial.supplemental_credit.is_some() {
            alberta_k5p(k1p_value, k2p_value).map_err(engine_message)?
        } else {
            zero
        };
        let provincial_cea = provincial.canada_employment_amount.unwrap_or(cea);
        let k4p_value = if req.province == Province::Yt {
            k4p(
                *provincial.lowest_rate.get(option),
                GrossEmploymentIncome::new(annual_gross),
                provincial_cea,
            )
            .map_err(engine_message)?
        } else {
            zero
        };
        let t4 = if provincial_exempt {
            zero
        } else {
            ontario_t4(
                a,
                provincial_brackets,
                k1p_value,
                k2p_value,
                zero,
                k4p_value,
                k5p_value,
            )
            .map_err(engine_message)?
        };
        let (v1, v2, y, s, t2) = provincial_assembly(
            req.province,
            a,
            t4,
            provincial,
            option,
            req.pay_period,
            req.lcp_purchase,
            req.dependants_disabled.unwrap_or(0),
            req.dependants_under_19.unwrap_or(0),
        )?;
        let lcp = match (req.lcp_purchase, provincial.lcp.as_ref()) {
            (Some(purchase), Some(params)) => round_tax_to_cent(
                purchase
                    .floor_at_zero()
                    .checked_mul_rate(params.rate)?
                    .min(params.max)
                    .checked_div(one)?,
            ),
            _ => zero,
        };
        let provincial_prorated =
            jurisdiction_prorated_rules(provincial, option) || provincial_bracket.prorated;
        (
            v,
            kp,
            k1p_value,
            k2p_value,
            t4,
            v1,
            v2,
            y,
            s,
            t2,
            lcp,
            k4p_value,
            k5p_value,
            provincial_prorated,
        )
    } else {
        (
            Rate::parse("0")?,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            zero,
            false,
        )
    };

    let (t1_period, t2_period, tb_federal, tb_provincial, tb) = if let Some(a_wo) = a_without {
        let bpaf_wo = resolve_basic_personal_amount(a_wo, fed_bpa_definition, None)
            .map_err(engine_message)?;
        let provincial_bpa_wo = match provincial {
            Some(jurisdiction) => resolve_basic_personal_amount(
                a_wo,
                jurisdiction.basic_personal_amount.get(option),
                Some(fed_bpa_definition),
            )
            .map_err(engine_message)?,
            None => zero,
        };
        let tc_wo = resolve_claim(req.federal_claim_code, req.federal_tc, bpaf_wo);
        let tcp_wo = resolve_claim(
            req.provincial_claim_code,
            req.provincial_tcp,
            provincial_bpa_wo,
        );
        let k1_wo = k1(federal_lowest, tc_wo).map_err(engine_message)?;
        let k2_wo = if option2 {
            k2_option2_at(federal_lowest)?
        } else {
            k2_from_parts(federal_lowest, false)?
        };
        let annual_gross_wo = if option2 {
            scale_by_s1(i_projected, s1)
                .map_err(engine_message)?
                .checked_add(req.ytd_bonus.unwrap_or(zero))?
        } else {
            i_for_a
                .checked_mul(p_money)?
                .checked_add(req.ytd_bonus.unwrap_or(zero))?
        };
        let k4_wo = k4(
            federal_lowest,
            GrossEmploymentIncome::new(annual_gross_wo),
            cea,
        )
        .map_err(engine_message)?;
        let t3_wo = if federal_exempt {
            zero
        } else {
            federal_t3(a_wo, k1_wo, k2_wo, zero, k4_wo, fed_brackets).map_err(engine_message)?
        };
        let t1_wo = if federal_exempt {
            zero
        } else {
            federal_t1(
                t3_wo,
                req.pay_period,
                req.lcf_purchase,
                lcf_params,
                req.province,
                fed.abatement,
                fed.surtax_flat,
            )
            .map_err(engine_message)?
        };
        let t2_wo = if let Some(provincial) = provincial {
            let provincial_brackets = provincial.brackets.get(option);
            let k1p_wo =
                k1p(*provincial.lowest_rate.get(option), tcp_wo).map_err(engine_message)?;
            let k2p_wo = if option2 {
                k2_option2_at(*provincial.lowest_rate.get(option))?
            } else {
                k2_from_parts(*provincial.lowest_rate.get(option), false)?
            };
            let k5p_wo = if provincial.supplemental_credit.is_some() {
                alberta_k5p(k1p_wo, k2p_wo).map_err(engine_message)?
            } else {
                zero
            };
            let provincial_cea = provincial.canada_employment_amount.unwrap_or(cea);
            let k4p_wo = if req.province == Province::Yt {
                k4p(
                    *provincial.lowest_rate.get(option),
                    GrossEmploymentIncome::new(annual_gross_wo),
                    provincial_cea,
                )
                .map_err(engine_message)?
            } else {
                zero
            };
            let t4_wo = if provincial_exempt {
                zero
            } else {
                ontario_t4(
                    a_wo,
                    provincial_brackets,
                    k1p_wo,
                    k2p_wo,
                    zero,
                    k4p_wo,
                    k5p_wo,
                )
                .map_err(engine_message)?
            };
            let (_v1, _v2, _y, _s, t2_wo) = provincial_assembly(
                req.province,
                a_wo,
                t4_wo,
                provincial,
                option,
                req.pay_period,
                req.lcp_purchase,
                req.dependants_disabled.unwrap_or(0),
                req.dependants_under_19.unwrap_or(0),
            )?;
            t2_wo
        } else {
            zero
        };
        let (tb_federal, tb_provincial) =
            if formulas::bonus::bonus_shortcut_applies(a).map_err(engine_message)? {
                (
                    formulas::bonus::bonus_shortcut_tax(non_periodic, req.province)
                        .map_err(engine_message)?,
                    zero,
                )
            } else {
                (
                    t1.checked_sub(t1_wo)?.floor_at_zero(),
                    t2.checked_sub(t2_wo)?.floor_at_zero(),
                )
            };
        let tb = tb_federal.checked_add(tb_provincial)?;
        (t1_wo, t2_wo, tb_federal, tb_provincial, tb)
    } else {
        (t1, t2, zero, zero, zero)
    };

    let federal_tax = if option2 {
        period_tax_option2(
            t1_period,
            zero,
            s1,
            m_federal,
            zero,
            additional_tax.checked_add(tb_federal)?,
        )
        .map_err(engine_message)?
    } else {
        formulas::per_period::per_period_tax(
            t1_period,
            zero,
            req.pay_period,
            additional_tax.checked_add(tb_federal)?,
            Granularity::Cent,
        )?
    };
    let provincial_tax = if option2 {
        period_tax_option2(zero, t2_period, s1, m_provincial, zero, tb_provincial)
            .map_err(engine_message)?
    } else {
        formulas::per_period::per_period_tax(
            zero,
            t2_period,
            req.pay_period,
            tb_provincial,
            Granularity::Cent,
        )?
    };
    // PDOC-facing total: sum of the two separately rounded lines. May differ
    // by 1¢ from breakdown.T = round((T1+T2)/P)+L (see data/factors.json).
    let total_tax = federal_tax.checked_add(provincial_tax)?;
    let total_deductions = total_tax
        .checked_add(c)?
        .checked_add(c2)?
        .checked_add(ei)?
        .checked_add(qpip)?
        .checked_add(union_dues)?;
    let mut warnings = if (federal_exempt || provincial_exempt) && v2 > zero {
        vec![Warning {
            code: "CLAIM_CODE_E_ONTARIO_HEALTH_PREMIUM".to_string(),
            message:
                "Claim code E removes income tax, but the Ontario Health Premium remains payable."
                    .to_string(),
        }]
    } else {
        vec![]
    };
    if let Some(warning) = proposed_rule_set_warning(set) {
        warnings.insert(0, warning);
    }
    if let Some(warning) = prorated_reconciliation_warning(req.province, req.as_of) {
        warnings.push(warning);
    }
    let prorated_rules_applied = provincial_prorated || fed_bracket.prorated;
    let t = if option2 {
        period_tax_option2(
            t1_period,
            t2_period,
            s1,
            m,
            m1,
            additional_tax.checked_add(tb)?,
        )
        .map_err(engine_message)?
    } else {
        formulas::per_period::per_period_tax(
            t1_period,
            t2_period,
            req.pay_period,
            additional_tax.checked_add(tb)?,
            Granularity::Cent,
        )?
    };

    Ok(Response {
        rule_set_version: set.rule_set_version.clone(),
        engine_version: Response::engine_version().to_string(),
        engine_build_sha256: Response::engine_build_sha256().to_string(),
        prorated_rules_applied,
        employee: EmployeeAmounts {
            federal_tax,
            provincial_tax,
            total_tax,
            cpp: c,
            cpp2: c2,
            ei,
            qpip,
            total_deductions,
            net_pay: req
                .cheque_gross()?
                .checked_sub(total_deductions)?
                .floor_at_zero(),
        },
        employer: EmployerAmounts {
            cpp: c,
            cpp2: c2,
            ei: employer_ei,
            qpip: zero,
        },
        annual_projection: AnnualProjection {
            taxable_income: a,
            federal_tax: t1,
            provincial_tax: t2,
            cpp: annual_cpp,
            cpp2: annual_cpp2,
            ei: annual_ei,
            qpip: annual_qpip,
        },
        breakdown: Breakdown {
            a,
            r,
            k,
            k1: k1_value,
            k2: k2_value,
            k4: k4_value,
            t3,
            t1,
            v,
            kp,
            k1p: k1p_value,
            k2p: k2p_value,
            t4,
            v1,
            v2,
            s,
            t2,
            t,
            tb,
            m,
            m1,
            c,
            c2,
            ei,
            f5: f5_value,
            bpaf,
            k3: zero,
            k3p: zero,
            k4p: k4p_value,
            k5p: k5p_value,
            lcf,
            lcp,
            y,
            f5a: f5a_value,
            f5b: f5b_value,
            d: ytd_cpp,
            d1: ytd_ei,
            d2: ytd_cpp2,
            p: p_count,
            pr: pr_count,
            pm: pm_count,
            s1,
            cea,
            tc,
            tcp,
            ie,
            qpip,
        },
        citations,
        warnings,
    })
}

fn scoped_is_split<T>(scoped: &crate::rules::schema::OptionScoped<T>) -> bool {
    matches!(scoped, crate::rules::schema::OptionScoped::PerOption { .. })
}

fn jurisdiction_prorated_rules(
    jurisdiction: &crate::rules::schema::Jurisdiction,
    option: crate::rules::schema::CalculationOption,
) -> bool {
    use crate::rules::schema::{CalculationOption, OptionScoped};
    let selected_prorated = jurisdiction.brackets.get(option).iter().any(|b| b.prorated);
    if option != CalculationOption::Option1 {
        return selected_prorated;
    }
    selected_prorated
        || scoped_is_split(&jurisdiction.brackets)
        || scoped_is_split(&jurisdiction.lowest_rate)
        || scoped_is_split(&jurisdiction.basic_personal_amount)
        || jurisdiction
            .tax_reduction
            .as_ref()
            .is_some_and(|s| matches!(s, OptionScoped::PerOption { .. }))
}

fn proposed_rule_set_warning(set: &crate::rules::schema::RuleSet) -> Option<Warning> {
    if set.status != crate::rules::schema::RuleSetStatus::Proposed {
        return None;
    }
    let date = set
        .announcement_date
        .map(|d| d.to_string())
        .unwrap_or_else(|| "unknown date".to_string());
    let source = set
        .announcement_source
        .as_deref()
        .unwrap_or("an unpublished announcement");
    Some(Warning {
        code: "RULE_SET_PROPOSED".to_string(),
        message: format!(
            "Rule set {} is proposed and not yet enacted. Announced {date} ({source}). The 2027 T4127 edition and 2027 YMPE have not been published. Do not file from this number.",
            set.rule_set_version
        ),
    })
}

fn prorated_reconciliation_warning(
    province: Province,
    as_of: crate::rules::schema::CalendarDate,
) -> Option<Warning> {
    let h2_2026 = as_of.year == 2026 && as_of.month >= 7;
    let changed = matches!(province, Province::Bc | Province::Nl | Province::Pe);
    if h2_2026 && changed {
        Some(Warning {
            code: "PRORATED_RECONCILIATION".to_string(),
            message: "Prorated provincial tax rules apply for the remainder of the year. Amounts withheld before 1 July 2026 may need year-end reconciliation.".to_string(),
        })
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn provincial_assembly(
    province: Province,
    a: Money,
    t4: Money,
    provincial: &crate::rules::schema::Jurisdiction,
    option: crate::rules::schema::CalculationOption,
    pay_period: crate::request::PayPeriod,
    lcp_purchase: Option<Money>,
    dependants_disabled: u8,
    dependants_under_19: u8,
) -> Result<(Money, Money, Money, Money, Money), EngineError> {
    use crate::formulas::province::bc::british_columbia_s;
    use crate::formulas::province::ontario::{
        ontario_s, ontario_t2, ontario_v1, ontario_v2, ontario_y,
    };
    let zero = Money::ZERO;
    match province {
        Province::On => {
            let v1 = ontario_v1(t4, provincial.surtax.as_deref().unwrap_or(&[]))
                .map_err(engine_message)?;
            let v2 = ontario_v2(
                a,
                provincial.health_premium.as_deref().ok_or_else(|| {
                    EngineError::Message("Ontario health-premium rules missing".to_string())
                })?,
            )
            .map_err(engine_message)?;
            let reduction = provincial
                .tax_reduction
                .as_ref()
                .map(|value| value.get(option))
                .ok_or_else(|| {
                    EngineError::Message("Ontario tax-reduction rules missing".to_string())
                })?;
            let y = ontario_y(
                dependants_disabled,
                dependants_under_19,
                reduction.dependant,
            )
            .map_err(engine_message)?;
            let s = ontario_s(t4, v1, y, reduction).map_err(engine_message)?;
            let t2 = ontario_t2(
                t4,
                v1,
                v2,
                s,
                pay_period,
                lcp_purchase,
                provincial.lcp.as_ref(),
            )
            .map_err(engine_message)?;
            Ok((v1, v2, y, s, t2))
        }
        Province::Bc => {
            let v1 = ontario_v1(t4, provincial.surtax.as_deref().unwrap_or(&[]))
                .map_err(engine_message)?;
            let s = match provincial.tax_reduction.as_ref() {
                Some(scoped) => {
                    british_columbia_s(a, t4, scoped.get(option)).map_err(engine_message)?
                }
                None => zero,
            };
            let t2 = ontario_t2(
                t4,
                v1,
                zero,
                s,
                pay_period,
                lcp_purchase,
                provincial.lcp.as_ref(),
            )
            .map_err(engine_message)?;
            Ok((v1, zero, zero, s, t2))
        }
        _ => {
            let v1 = ontario_v1(t4, provincial.surtax.as_deref().unwrap_or(&[]))
                .map_err(engine_message)?;
            let v2 = match provincial.health_premium.as_deref() {
                Some(tiers) => ontario_v2(a, tiers).map_err(engine_message)?,
                None => zero,
            };
            let t2 = ontario_t2(
                t4,
                v1,
                v2,
                zero,
                pay_period,
                lcp_purchase,
                provincial.lcp.as_ref(),
            )
            .map_err(engine_message)?;
            Ok((v1, v2, zero, zero, t2))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calculate, list_jurisdictions, ClaimCode, EngineError, Money, PayPeriod, Province, Request,
        Response, QUEBEC_UNSUPPORTED_REASON,
    };
    use crate::decimal::Rate;
    use crate::formulas::bonus::{self as bonus_formulas, BonusIncomeInputs};
    use crate::formulas::federal_tax::federal_t3;
    use crate::formulas::province::alberta::alberta_k5p;
    use crate::request::K2Method;
    use crate::rules::loader::{
        load_ruleset_2026_01_01, load_ruleset_2026_07_01, EMBEDDED_REGISTRY,
    };
    use crate::rules::schema::{CalculationOption, CalendarDate, JurisdictionCode};
    use proptest::prelude::*;
    use std::str::FromStr;

    fn money(s: &str) -> Money {
        Money::parse(s).unwrap()
    }

    fn ontario_weekly_1000() -> Request {
        Request {
            as_of: CalendarDate::from_str("2026-01-01").unwrap(),
            province: Province::On,
            pay_period: PayPeriod::new(52).unwrap(),
            gross_pay: money("1000.00"),
            calculation_option: CalculationOption::Option1,
            pensionable_earnings: None,
            insurable_earnings: None,
            federal_claim_code: Some(ClaimCode::Code(1)),
            provincial_claim_code: Some(ClaimCode::Code(1)),
            federal_tc: None,
            provincial_tcp: None,
            cpp_months: 12,
            k2_method: K2Method::PdocObserved,
            bonus_method: crate::request::BonusMethod::Regular,
            rounding_compat: crate::request::RoundingCompat::T4127,
            ytd_pensionable_earnings: None,
            ytd_insurable_earnings: None,
            ytd_cpp: None,
            ytd_cpp2: None,
            ytd_ei: None,
            ytd_federal_tax: None,
            ytd_provincial_tax: None,
            pay_periods_elapsed: None,
            bonus: None,
            retroactive_pay: None,
            ytd_bonus: None,
            f5b_ytd: None,
            most_recent_i: None,
            rpp: None,
            bonus_rrsp: None,
            ytd_bonus_rrsp: None,
            ytd_income: None,
            ytd_rpp: None,
            ytd_union_dues: None,
            f5a_ytd: None,
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
        }
    }

    fn cra_bonus_request() -> Request {
        let mut req = ontario_weekly_1000();
        req.bonus = Some(money("2500.00"));
        req.ytd_bonus = Some(money("1500.00"));
        req.f5b_ytd = Some(money("14.60"));
        req.pay_periods_elapsed = Some(29);
        req
    }

    /// 30. CRA T4127 Chapter 4 regular-bonus worked example. Must pass exactly.
    #[test]
    fn cra_regular_bonus_worked_example_golden_vector() {
        let resp = calculate(&cra_bonus_request(), &EMBEDDED_REGISTRY).unwrap();
        let b = &resp.breakdown;
        assert_eq!(b.c, money("204.25"), "C");
        assert_eq!(b.f5, money("34.33"), "F5");
        assert_eq!(b.f5a, money("9.81"), "F5A");
        assert_eq!(b.f5b, money("24.52"), "F5B");
        assert_eq!(b.a, money("55450.76"), "A(with)");
        assert_eq!(b.k1, money("2303.28"), "K1");
        assert_eq!(b.k2, money("491.63"), "K2(with)");
        assert_eq!(b.k4, money("210.14"), "K4");
        assert_eq!(b.t3, money("4758.06"), "T3(with)");

        let groups = bonus_formulas::regular_bonus_groups(&BonusIncomeInputs {
            pay_period: PayPeriod::new(52).unwrap(),
            periods_remaining: 23,
            periodic_income: money("1000.00"),
            rpp: money("0.00"),
            alimony_pre_1997: money("0.00"),
            f5a: b.f5a,
            union_dues: money("0.00"),
            prescribed_zone: money("0.00"),
            annual_deductions: money("0.00"),
            current_non_periodic: money("2500.00"),
            bonus_rrsp: money("0.00"),
            f5b: b.f5b,
            prior_non_periodic: money("1500.00"),
            prior_bonus_rrsp: money("0.00"),
            f5b_ytd: money("14.60"),
            ytd_periodic_income: money("0.00"),
            ytd_rpp: money("0.00"),
            ytd_f5a: money("0.00"),
            ytd_union_dues: money("0.00"),
        })
        .unwrap();
        assert_eq!(groups.a_without().unwrap(), money("52975.28"), "A(without)");
        let set = load_ruleset_2026_01_01().expect("2026-01-01 rules");
        let k2_wo = bonus_formulas::k2_non_periodic(
            Rate::parse("0.14").unwrap(),
            PayPeriod::new(52).unwrap(),
            money("55.50"),
            money("148.75"),
            money("89.25"),
            money("16.30"),
            money("40.75"),
            money("24.45"),
            false,
            &set.cpp,
            &set.ei,
        )
        .unwrap();
        assert_eq!(k2_wo, money("468.60"), "K2(without)");
        let t3_wo = federal_t3(
            groups.a_without().unwrap(),
            b.k1,
            k2_wo,
            money("0.00"),
            b.k4,
            set.jurisdictions
                .get(&JurisdictionCode("FED".to_string()))
                .unwrap()
                .brackets
                .get(CalculationOption::Option1),
        )
        .unwrap();
        assert_eq!(t3_wo, money("4434.52"), "T3(without)");
        assert_eq!(
            b.t3.checked_sub(t3_wo).unwrap(),
            money("323.54"),
            "published federal TB"
        );
    }

    /// 32. $5,000 shortcut boundary on A with the bonus.
    #[test]
    fn five_thousand_shortcut_boundary_through_calculate() {
        let mut base = ontario_weekly_1000();
        base.gross_pay = money("0.00");
        base.most_recent_i = Some(money("0.00"));
        base.cpp_exempt = Some(true);
        base.federal_claim_code = Some(ClaimCode::Code(0));
        base.provincial_claim_code = Some(ClaimCode::Code(0));

        let mut at = base.clone();
        at.bonus = Some(money("5000.00"));
        let at_resp = calculate(&at, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(at_resp.breakdown.a, money("5000.00"));
        assert_eq!(at_resp.breakdown.tb, money("750.00"));

        let mut under = base.clone();
        under.bonus = Some(money("4999.99"));
        let under_resp = calculate(&under, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(under_resp.breakdown.a, money("4999.99"));
        assert_eq!(under_resp.breakdown.tb, money("750.00"));

        let mut over = base;
        over.bonus = Some(money("5000.01"));
        let over_resp = calculate(&over, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(over_resp.breakdown.a, money("5000.01"));
        assert_ne!(
            over_resp.breakdown.tb,
            money("750.00"),
            "$5,000.01 must not take the 15% shortcut"
        );
    }

    /// 33. F5A and F5B from the with-bonus PI split are reused for A(without).
    #[test]
    fn f5a_f5b_not_recomputed_on_without_bonus_pass() {
        let resp = calculate(&cra_bonus_request(), &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(resp.breakdown.f5a, money("9.81"));
        assert_eq!(resp.breakdown.f5b, money("24.52"));
        let expected_without = money("52")
            .checked_mul(money("1000.00").checked_sub(resp.breakdown.f5a).unwrap())
            .unwrap()
            .checked_add(money("1500.00"))
            .unwrap()
            .checked_sub(money("14.60"))
            .unwrap();
        assert_eq!(expected_without, money("52975.28"));
        assert_eq!(
            resp.breakdown.a.checked_sub(expected_without).unwrap(),
            money("2500.00").checked_sub(resp.breakdown.f5b).unwrap()
        );
    }

    /// 34. I = 0 uses most_recent_i in the regular group.
    #[test]
    fn bonus_only_period_uses_most_recent_i() {
        let mut req = cra_bonus_request();
        req.gross_pay = money("0.00");
        req.most_recent_i = Some(money("1000.00"));
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        let regular = money("52")
            .checked_mul(money("1000.00").checked_sub(resp.breakdown.f5a).unwrap())
            .unwrap();
        let current = money("2500.00").checked_sub(resp.breakdown.f5b).unwrap();
        let prior = money("1500.00").checked_sub(money("14.60")).unwrap();
        assert_eq!(
            resp.breakdown.a,
            regular
                .checked_add(current)
                .unwrap()
                .checked_add(prior)
                .unwrap()
        );
        assert!(resp.breakdown.a > money("50000.00"));
    }

    /// 36. retroactive_pay uses the same path as bonus.
    #[test]
    fn retroactive_pay_matches_equal_bonus() {
        let mut as_bonus = ontario_weekly_1000();
        as_bonus.bonus = Some(money("1500.00"));
        let mut as_retro = ontario_weekly_1000();
        as_retro.retroactive_pay = Some(money("1500.00"));
        let bonus_resp = calculate(&as_bonus, &EMBEDDED_REGISTRY).unwrap();
        let retro_resp = calculate(&as_retro, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(bonus_resp.breakdown.a, retro_resp.breakdown.a);
        assert_eq!(bonus_resp.breakdown.tb, retro_resp.breakdown.tb);
        assert_eq!(bonus_resp.breakdown.f5a, retro_resp.breakdown.f5a);
        assert_eq!(bonus_resp.breakdown.f5b, retro_resp.breakdown.f5b);
        assert_eq!(
            bonus_resp.employee.federal_tax,
            retro_resp.employee.federal_tax
        );
        assert_eq!(bonus_resp.employee.net_pay, retro_resp.employee.net_pay);
    }

    // 114. A negative → T = L exactly; response still carries a full zeroed
    // breakdown (not an error).
    #[test]
    fn negative_annual_taxable_income_yields_t_equals_l_not_error() {
        let mut req = ontario_weekly_1000();
        // Huge annual deductions force A < 0.
        req.prescribed_zone_deduction = Some(money("999999.00"));
        req.additional_tax_requested = Some(money("12.34"));
        let resp = calculate(&req, &EMBEDDED_REGISTRY).expect("negative A is Ok, not Err");
        let tax = resp
            .employee
            .federal_tax
            .checked_add(resp.employee.provincial_tax)
            .unwrap();
        assert_eq!(tax, money("12.34"));
        // Full breakdown present; annual tax factors zeroed.
        assert_eq!(resp.breakdown.t1, money("0.00"));
        assert_eq!(resp.breakdown.t2, money("0.00"));
        assert_eq!(resp.breakdown.t3, money("0.00"));
        assert_eq!(resp.breakdown.t4, money("0.00"));
    }

    /// 117. End-to-end Ontario weekly $1000, claim code 1, PM=12, no YTD.
    ///
    /// Literals use `round_contribution_to_cent` for C (CRA worked example at
    /// §6.13 also prints C = $55.50 for weekly $1,000). Step 5 hand-figure
    /// disagreement (55.49 vs 55.50) is resolved here by the published example.
    #[test]
    fn ontario_weekly_1000_claim_code_1_complete_response() {
        let resp = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();

        assert_eq!(resp.rule_set_version, "2026-01-01");
        assert_eq!(resp.employee.cpp, money("55.50"));
        assert_eq!(resp.employee.cpp2, money("0.00"));
        assert_eq!(resp.employee.ei, money("16.30"));
        assert_eq!(resp.employee.qpip, money("0.00"));
        assert_eq!(resp.employee.federal_tax, money("81.61"));
        assert_eq!(resp.employee.provincial_tax, money("45.80"));
        assert_eq!(resp.employee.total_deductions, money("199.21"));
        assert_eq!(resp.employee.net_pay, money("800.79"));

        assert_eq!(resp.employer.cpp, money("55.50"));
        assert_eq!(resp.employer.cpp2, money("0.00"));
        assert_eq!(resp.employer.ei, money("22.82"));
        assert_eq!(resp.employer.qpip, money("0.00"));

        assert_eq!(resp.annual_projection.taxable_income, money("51514.84"));
        assert_eq!(resp.annual_projection.federal_tax, money("4243.86"));
        assert_eq!(resp.annual_projection.provincial_tax, money("2381.51"));
        // Annualised contributions (P × period, capped at annual max).
        assert_eq!(resp.annual_projection.cpp, money("2886.00")); // 52 × 55.50
        assert_eq!(resp.annual_projection.cpp2, money("0.00"));
        assert_eq!(resp.annual_projection.ei, money("847.60")); // 52 × 16.30
        assert_eq!(resp.annual_projection.qpip, money("0.00"));

        let b = &resp.breakdown;
        assert_eq!(b.a, money("51514.84"));
        assert_eq!(b.r, Rate::parse("0.1400").unwrap());
        assert_eq!(b.k, money("0.00"));
        assert_eq!(b.k1, money("2303.28"));
        assert_eq!(b.k2, money("454.80"));
        assert_eq!(b.k3, money("0.00"));
        assert_eq!(b.k4, money("210.14"));
        assert_eq!(b.lcf, money("0.00"));
        assert_eq!(b.t3, money("4243.86"));
        assert_eq!(b.t1, money("4243.86"));
        assert_eq!(b.v, Rate::parse("0.0505").unwrap());
        assert_eq!(b.kp, money("0.00"));
        assert_eq!(b.k1p, money("655.94"));
        assert_eq!(b.k2p, money("164.05"));
        assert_eq!(b.k3p, money("0.00"));
        assert_eq!(b.k4p, money("0.00"));
        assert_eq!(b.lcp, money("0.00"));
        assert_eq!(b.t4, money("1781.51"));
        assert_eq!(b.v1, money("0.00"));
        assert_eq!(b.v2, money("600.00"));
        assert_eq!(b.s, money("0.00"));
        assert_eq!(b.t2, money("2381.51"));
        assert_eq!(b.c, money("55.50"));
        assert_eq!(b.c2, money("0.00"));
        assert_eq!(b.ei, money("16.30"));
        assert_eq!(b.ie, money("1000.00"));
        assert_eq!(b.f5, money("9.33"));
        assert_eq!(b.tc, money("16452.00"));
        assert_eq!(b.tcp, money("12989.00"));
        assert_eq!(b.bpaf, money("16452.00"));
        assert_eq!(b.cea, money("1501.00"));
        assert_eq!(b.t, money("127.41"));
        assert_eq!(b.k5p, money("0.00"));
        assert_eq!(b.y, money("0.00"));
        assert_eq!(b.f5a, money("9.33"));
        assert_eq!(b.f5b, money("0.00"));
        assert_eq!(b.d, money("0.00"));
        assert_eq!(b.d1, money("0.00"));
        assert_eq!(b.d2, money("0.00"));
        assert_eq!(b.p, crate::response::Count::new(52));
        assert_eq!(b.pr, crate::response::Count::new(52));
        assert_eq!(b.pm, crate::response::Count::new(12));
        assert_eq!(b.qpip, money("0.00"));

        let encoded = serde_json::to_value(&resp).unwrap();
        assert_eq!(encoded["breakdown"]["A"], "51514.84");
        assert_eq!(encoded["breakdown"]["T"], "127.41");
        assert_eq!(encoded["breakdown"]["P"], "52");
        assert_eq!(encoded["breakdown"]["PR"], "52");
        assert_eq!(encoded["breakdown"]["PM"], "12");
        assert!(encoded["breakdown"].get("a").is_none());
    }

    /// T4127 Y = 554 × dependants under 19. The request field is the cutoff, not a province prefix.
    #[test]
    fn dependants_under_19_feeds_factor_y() {
        let mut req = ontario_weekly_1000();
        req.dependants_under_19 = Some(1);
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(resp.breakdown.y, money("554.00"));
        req.dependants_disabled = Some(1);
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(resp.breakdown.y, money("1108.00"));
    }

    /// 118. Response rule_set_version matches the set resolved from as_of.
    #[test]
    fn rule_set_version_matches_resolved_as_of() {
        let req = ontario_weekly_1000();
        let set = EMBEDDED_REGISTRY.resolve(req.as_of).unwrap();
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(resp.rule_set_version, set.rule_set_version);
    }

    /// 119. prorated_rules_applied is false (empty) for a 2026-01-01 as_of.
    #[test]
    fn prorated_rules_applied_false_on_2026_01_01() {
        let resp = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();
        assert!(!resp.prorated_rules_applied);
    }

    /// 120. citations is non-empty; every rule-sourced breakdown factor has one.
    #[test]
    fn citations_cover_rule_sourced_breakdown_factors() {
        let resp = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();
        assert!(!resp.citations.is_empty());
        let cited: std::collections::BTreeSet<&str> =
            resp.citations.iter().map(|c| c.factor.as_str()).collect();
        for factor in ["R", "K", "V", "KP", "CEA", "BPAF", "TC", "TCP"] {
            assert!(
                cited.contains(factor),
                "missing citation for rule-sourced factor {factor}; have {cited:?}"
            );
            assert!(
                crate::response::Breakdown::FACTOR_KEYS.contains(&factor),
                "citation factor {factor} is not a T4127 breakdown key"
            );
        }
    }

    /// 121. warnings is empty for a plain case.
    #[test]
    fn warnings_empty_for_plain_case() {
        let resp = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();
        assert!(resp.warnings.is_empty());
    }

    /// 122. DETERMINISM: same request serialised twice → byte-identical JSON.
    #[test]
    fn calculate_response_json_is_byte_identical() {
        let req = ontario_weekly_1000();
        let a = serde_json::to_string(&calculate(&req, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        let b = serde_json::to_string(&calculate(&req, &EMBEDDED_REGISTRY).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    /// Headline tax is the sum of separately rounded federal and provincial
    /// lines (`employee.total_tax`). That can differ by 1¢ from `breakdown.T`
    /// = round((T1+T2)/P)+L; both are correct under their semantics.
    #[test]
    fn headline_tax_reconciles_to_breakdown() {
        let resp = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();
        let p = Money::parse("52").unwrap();
        let quotient = crate::rounding::round_tax_to_cent(
            resp.breakdown
                .t1
                .checked_add(resp.breakdown.t2)
                .unwrap()
                .checked_div(p)
                .unwrap(),
        );
        let l = money("0.00");
        let from_breakdown = quotient.checked_add(l).unwrap();
        assert_eq!(resp.breakdown.t, from_breakdown, "breakdown.T is Step 6");
        assert_eq!(
            resp.employee.total_tax,
            resp.employee
                .federal_tax
                .checked_add(resp.employee.provincial_tax)
                .unwrap(),
            "total_tax is the sum of the two rounded lines"
        );
        let gap = if resp.employee.total_tax >= from_breakdown {
            resp.employee.total_tax.checked_sub(from_breakdown).unwrap()
        } else {
            from_breakdown.checked_sub(resp.employee.total_tax).unwrap()
        };
        assert!(
            gap <= money("0.01"),
            "total_tax {} vs breakdown.T {} differs by {gap}",
            resp.employee.total_tax,
            from_breakdown
        );
    }

    /// L is inside `employee.federal_tax` (T4127 Step 6), not a sibling line.
    #[test]
    fn additional_tax_is_inside_federal_tax() {
        let base = calculate(&ontario_weekly_1000(), &EMBEDDED_REGISTRY).unwrap();
        let mut req = ontario_weekly_1000();
        req.additional_tax_requested = Some(money("25.00"));
        let with_l = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(
            with_l.employee.federal_tax,
            base.employee
                .federal_tax
                .checked_add(money("25.00"))
                .unwrap()
        );
        assert_eq!(with_l.employee.provincial_tax, base.employee.provincial_tax);
    }

    /// U1 reduces A and is also withheld: it appears in total_deductions.
    #[test]
    fn union_dues_are_withheld_in_total_deductions() {
        let mut req = ontario_weekly_1000();
        req.union_dues = Some(money("15.00"));
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        let withholdings = resp
            .employee
            .federal_tax
            .checked_add(resp.employee.provincial_tax)
            .unwrap()
            .checked_add(resp.employee.cpp)
            .unwrap()
            .checked_add(resp.employee.cpp2)
            .unwrap()
            .checked_add(resp.employee.ei)
            .unwrap()
            .checked_add(resp.employee.qpip)
            .unwrap();
        assert_eq!(
            resp.employee.total_deductions,
            withholdings.checked_add(money("15.00")).unwrap()
        );
        assert_eq!(
            resp.employee.net_pay,
            req.gross_pay
                .checked_sub(resp.employee.total_deductions)
                .unwrap()
        );
    }

    /// 124. net_pay == gross_pay − total_deductions, exactly.
    #[test]
    fn net_pay_equals_gross_minus_total_deductions() {
        let req = ontario_weekly_1000();
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(
            resp.employee.net_pay,
            req.gross_pay
                .checked_sub(resp.employee.total_deductions)
                .unwrap()
        );
    }

    /// D-003 regression: weekly $2000, zero YTD → C2 = 0 (literal W form).
    /// Also pins the lowest-rate credit path (K1/K2/K4 use 0.14, not bracket R).
    #[test]
    fn weekly_2000_zero_ytd_cpp2_is_zero() {
        let mut req = ontario_weekly_1000();
        req.as_of = CalendarDate::from_str("2026-01-15").unwrap();
        req.gross_pay = money("2000.00");
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(resp.employee.cpp2, money("0.00"));
        assert_eq!(resp.employee.cpp, money("115.00"));
        assert_eq!(resp.employee.ei, money("32.60"));
        assert_eq!(resp.breakdown.r, Rate::parse("0.2050").unwrap());
        assert_eq!(resp.breakdown.k1, money("2303.28")); // 0.14 × 16452, not 0.205 ×
        assert_eq!(resp.breakdown.k4, money("210.14")); // 0.14 × 1501
        assert_eq!(resp.employee.federal_tax, money("272.05"));
        assert_eq!(resp.employee.provincial_tax, money("137.98"));
        assert_eq!(resp.employee.total_deductions, money("557.63"));
        assert_eq!(resp.employee.net_pay, money("1442.37"));
    }

    /// Spec §21.7 / test 101: claim code E → income tax T = 0, but Ontario V2
    /// still payable and present in the response when A > 20_000.
    #[test]
    fn claim_code_e_still_pays_ontario_health_premium() {
        let mut req = ontario_weekly_1000();
        req.federal_claim_code = Some(ClaimCode::E);
        req.provincial_claim_code = Some(ClaimCode::E);
        // Raise income so A > 20000 with certainty.
        req.gross_pay = money("2500.00");
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert!(resp.breakdown.a > money("20000.00"));
        assert!(resp.breakdown.v2 > money("0.00"));
        assert_eq!(resp.breakdown.t4, money("0.00"));
        // OHP flows into provincial tax / T2 path.
        assert!(resp.employee.provincial_tax > money("0.00") || resp.breakdown.t2 > money("0.00"));
        assert!(
            resp.warnings
                .iter()
                .any(|w| w.code.contains("E") || w.message.contains("Health")),
            "claim-code E + OHP should warn; got {:?}",
            resp.warnings
        );
    }

    // 125. Property (spec §16.2), ≥10k generated Ontario requests.
    //
    // Tax vs gross is not required to be strictly monotone: published-K
    // discontinuities (M-001) can drop annual T3 by up to $1.00, which is under
    // two cents at P=52. The assertion below is the M-001-bounded form.
    // `#[ignore]` is never a resolution for this test.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10_000))]
        #[test]
        fn ontario_calculate_invariants(
            gross_cents in 1u64..=5_000_000u64,
            gross_hi_cents in 1u64..=5_000_000u64,
            periods in prop::sample::select(vec![12u16, 24, 26, 52]),
            claim in 0u8..=1u8,
        ) {
            let registry = &*EMBEDDED_REGISTRY;
            let mk = |cents: u64| {
                let mut r = ontario_weekly_1000();
                r.gross_pay = money(&format!("{}.{:02}", cents / 100, cents % 100));
                r.pay_period = PayPeriod::new(periods).unwrap();
                r.federal_claim_code = Some(ClaimCode::Code(claim));
                r.provincial_claim_code = Some(ClaimCode::Code(claim));
                r
            };
            let lo = gross_cents.min(gross_hi_cents);
            let hi = gross_cents.max(gross_hi_cents);
            let req_lo = mk(lo);
            let req_hi = mk(hi);
            let resp_lo = calculate(&req_lo, registry).unwrap();
            let resp_hi = calculate(&req_hi, registry).unwrap();

            prop_assert!(resp_lo.employee.total_deductions <= req_lo.gross_pay);
            prop_assert!(resp_hi.employee.total_deductions <= req_hi.gross_pay);
            prop_assert!(!resp_lo.employee.net_pay.is_negative());
            prop_assert!(!resp_hi.employee.net_pay.is_negative());
            for resp in [&resp_lo, &resp_hi] {
                prop_assert!(!resp.employee.federal_tax.is_negative());
                prop_assert!(!resp.employee.provincial_tax.is_negative());
                prop_assert!(!resp.employee.cpp.is_negative());
                prop_assert!(!resp.employee.cpp2.is_negative());
                prop_assert!(!resp.employee.ei.is_negative());
                prop_assert!(!resp.employee.qpip.is_negative());
            }
            let tax = |r: &Response| {
                r.employee
                    .federal_tax
                    .checked_add(r.employee.provincial_tax)
                    .unwrap()
            };
            let tax_lo = tax(&resp_lo);
            let tax_hi = tax(&resp_hi);
            if tax_lo <= tax_hi {
                prop_assert!(true);
            } else {
                let drop = tax_lo.checked_sub(tax_hi).unwrap();
                // M-001: annual jump ≤ $1.00 → per-period at P≥12 is ≤ ~$0.09;
                // at weekly the measured bound is under two cents. Use $0.09 so
                // monthly (P=12) cases remain honest without re-introducing a
                // false one-cent continuity claim.
                prop_assert!(
                    drop <= money("0.09"),
                    "period tax dropped more than the M-001 bound: {tax_lo} → {tax_hi} (gross {lo}→{hi}, P={periods})"
                );
            }
        }
    }

    // 125b. Increasing TC/TCP never increases tax.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2_000))]
        #[test]
        fn increasing_credits_never_increases_tax(
            gross_cents in 10_000u64..=2_000_000u64,
            tc_lo in 0u64..=1_645_200u64,
            tc_delta in 0u64..=500_000u64,
        ) {
            let registry = &*EMBEDDED_REGISTRY;
            let base = || {
                let mut r = ontario_weekly_1000();
                r.gross_pay = money(&format!("{}.{:02}", gross_cents / 100, gross_cents % 100));
                r.federal_claim_code = None;
                r.provincial_claim_code = None;
                r
            };
            let tc_a = money(&format!("{}.{:02}", tc_lo / 100, tc_lo % 100));
            let tc_b = tc_a
                .checked_add(money(&format!("{}.{:02}", tc_delta / 100, tc_delta % 100)))
                .unwrap();
            let mut ra = base();
            ra.federal_tc = Some(tc_a);
            ra.provincial_tcp = Some(tc_a);
            let mut rb = base();
            rb.federal_tc = Some(tc_b);
            rb.provincial_tcp = Some(tc_b);
            let a = calculate(&ra, registry).unwrap();
            let b = calculate(&rb, registry).unwrap();
            let tax = |r: &Response| {
                r.employee
                    .federal_tax
                    .checked_add(r.employee.provincial_tax)
                    .unwrap()
            };
            prop_assert!(tax(&b) <= tax(&a));
        }
    }

    /// Period tax may drop by at most the M-001 weekly bound (~2¢) when annual
    /// A crosses a published-K discontinuity. Strict one-cent continuity is false.
    #[test]
    fn period_tax_near_k_discontinuities_is_m001_bounded() {
        let registry = &*EMBEDDED_REGISTRY;
        // Weekly gross bands that put annual A near federal thresholds 58523, 117045, 181440.
        let bands = [(1100u64, 1150u64), (2200, 2300), (3450, 3600)];
        let mut drops = Vec::new();
        for (lo, hi) in bands {
            let mut prev: Option<Money> = None;
            for dollar in lo..=hi {
                for frac in 0..100u64 {
                    let mut req = ontario_weekly_1000();
                    req.gross_pay = money(&format!("{dollar}.{frac:02}"));
                    req.federal_claim_code = Some(ClaimCode::Code(0));
                    req.provincial_claim_code = Some(ClaimCode::Code(0));
                    let resp = calculate(&req, registry).unwrap();
                    let tax = resp
                        .employee
                        .federal_tax
                        .checked_add(resp.employee.provincial_tax)
                        .unwrap();
                    if let Some(p) = prev {
                        if tax < p {
                            let drop = p.checked_sub(tax).unwrap();
                            drops.push((req.gross_pay, p, tax, drop));
                            assert!(
                                drop <= money("0.02"),
                                "period tax dropped more than M-001 weekly bound at {}: {} → {} (Δ {})",
                                req.gross_pay,
                                p,
                                tax,
                                drop
                            );
                        }
                    }
                    prev = Some(tax);
                }
            }
        }
        eprintln!("period-tax drops found: {}", drops.len());
        for (g, p, t, d) in &drops {
            eprintln!("  gross={g} tax {p} → {t} (drop {d})");
        }
    }

    #[test]
    fn calculate_signature_returns_engine_error_on_bad_as_of() {
        let mut req = ontario_weekly_1000();
        req.as_of = CalendarDate::from_str("1990-01-01").unwrap();
        let err = calculate(&req, &EMBEDDED_REGISTRY).unwrap_err();
        assert!(matches!(err, EngineError::Rule(_)));
    }

    fn date(s: &str) -> CalendarDate {
        CalendarDate::from_str(s).unwrap()
    }

    fn request_at(province: Province, as_of: &str, gross: &str) -> Request {
        let mut req = ontario_weekly_1000();
        req.province = province;
        req.as_of = date(as_of);
        req.gross_pay = money(gross);
        req
    }

    fn has_prorated_warning(resp: &Response) -> bool {
        resp.warnings
            .iter()
            .any(|w| w.code == "PRORATED_RECONCILIATION")
    }

    /// Test 20 — identical BC request, as_of 2026-06-30 vs 2026-07-01.
    #[test]
    fn bc_june_vs_july_boundary_changes_tax_and_proration_flag() {
        let june = calculate(
            &request_at(Province::Bc, "2026-06-30", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        let july = calculate(
            &request_at(Province::Bc, "2026-07-01", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert_ne!(june.employee.provincial_tax, july.employee.provincial_tax);
        assert_eq!(june.rule_set_version, "2026-01-01");
        assert_eq!(july.rule_set_version, "2026-07-01");
        assert!(!june.prorated_rules_applied);
        assert!(july.prorated_rules_applied);
    }

    /// Test 21 — the same as_of pair for NL and PE.
    #[test]
    fn nl_and_pe_june_vs_july_boundary() {
        for (province, gross) in [(Province::Nl, "1000.00"), (Province::Pe, "8000.00")] {
            let june = calculate(
                &request_at(province, "2026-06-30", gross),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            let july = calculate(
                &request_at(province, "2026-07-01", gross),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            assert_ne!(
                june.employee.provincial_tax, july.employee.provincial_tax,
                "{province:?} provincial tax must change at the July boundary"
            );
            assert_ne!(june.rule_set_version, july.rule_set_version);
            assert!(!june.prorated_rules_applied);
            assert!(july.prorated_rules_applied);
        }
    }

    /// Test 22 — Ontario is unchanged across the boundary; proration must not leak.
    #[test]
    fn ontario_june_vs_july_is_identical_and_not_prorated() {
        let june = calculate(
            &request_at(Province::On, "2026-06-30", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        let july = calculate(
            &request_at(Province::On, "2026-07-01", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert_eq!(june.employee, july.employee);
        assert_eq!(june.annual_projection, july.annual_projection);
        assert_eq!(june.breakdown, july.breakdown);
        assert!(!june.prorated_rules_applied);
        assert!(!july.prorated_rules_applied);
        assert_ne!(june.rule_set_version, july.rule_set_version);
    }

    /// Test 23 / 42 — THE PRORATION TEST. Identical BC/NL/PE request on
    /// 2026-08-01 under Option 1 vs Option 2: Option 1 uses the prorated
    /// mid-year figures; Option 2 uses the true July figures; they differ;
    /// `prorated_rules_applied` is set only for Option 1.
    #[test]
    fn option1_vs_option2_proration_is_why_option_scoped_exists() {
        use crate::rules::schema::OptionScoped;

        let mut bc1 = request_at(Province::Bc, "2026-08-01", "800.00");
        let mut bc2 = bc1.clone();
        bc1.calculation_option = CalculationOption::Option1;
        bc2.calculation_option = CalculationOption::Option2;
        let r1 = calculate(&bc1, &EMBEDDED_REGISTRY).unwrap();
        let r2 = calculate(&bc2, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(r1.breakdown.v, Rate::parse("0.0614").unwrap());
        assert_eq!(r2.breakdown.v, Rate::parse("0.0560").unwrap());
        assert_ne!(r1.breakdown.v, r2.breakdown.v);
        assert_ne!(r1.employee.provincial_tax, r2.employee.provincial_tax);
        assert!(
            r1.prorated_rules_applied,
            "Option 1 must report prorated_rules_applied"
        );
        assert!(
            !r2.prorated_rules_applied,
            "Option 2 uses true rates; prorated_rules_applied must be empty"
        );
        let july = load_ruleset_2026_07_01().unwrap();
        let bc = july
            .jurisdictions
            .get(&JurisdictionCode("BC".to_string()))
            .unwrap();
        let OptionScoped::PerOption {
            option1: s2_opt1,
            option2: s2_opt2,
        } = bc.tax_reduction.as_ref().expect("BC tax_reduction")
        else {
            panic!("BC July tax_reduction must be PerOption (805 vs 690)");
        };
        assert_eq!(s2_opt1.basic, money("805.00"));
        assert_eq!(s2_opt2.basic, money("690.00"));
        assert_ne!(r1.breakdown.s, r2.breakdown.s);

        let mut nl1 = request_at(Province::Nl, "2026-08-01", "1000.00");
        let mut nl2 = nl1.clone();
        nl1.calculation_option = CalculationOption::Option1;
        nl2.calculation_option = CalculationOption::Option2;
        let n1 = calculate(&nl1, &EMBEDDED_REGISTRY).unwrap();
        let n2 = calculate(&nl2, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(n1.breakdown.tcp, money("15000.00"));
        assert_eq!(n2.breakdown.tcp, money("13094.00"));
        assert_ne!(n1.breakdown.tcp, n2.breakdown.tcp);
        assert!(n1.prorated_rules_applied);
        assert!(!n2.prorated_rules_applied);

        let mut pe1 = request_at(Province::Pe, "2026-08-01", "8000.00");
        let mut pe2 = pe1.clone();
        pe1.calculation_option = CalculationOption::Option1;
        pe2.calculation_option = CalculationOption::Option2;
        let p1 = calculate(&pe1, &EMBEDDED_REGISTRY).unwrap();
        let p2 = calculate(&pe2, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(p1.breakdown.v, Rate::parse("0.2100").unwrap());
        assert_eq!(p2.breakdown.v, Rate::parse("0.2000").unwrap());
        assert_ne!(p1.breakdown.v, p2.breakdown.v);
        assert!(p1.prorated_rules_applied);
        assert!(!p2.prorated_rules_applied);
    }

    /// 43. calculate() resets S1 to 52/1 when Option 2 starts mid-year (elapsed omitted).
    #[test]
    fn option2_s1_resets_to_period_1_when_elapsed_omitted_mid_year() {
        let mut req = request_at(Province::On, "2026-07-08", "1000.00");
        req.calculation_option = CalculationOption::Option2;
        let reset = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(reset.breakdown.s1.to_string(), "52/1");
        req.pay_periods_elapsed = Some(26);
        let continued = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(continued.breakdown.s1.to_string(), "52/27");
        req.pay_periods_elapsed = Some(1);
        let period2 = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(period2.breakdown.s1.to_string(), "52/2");
    }

    /// Option 2 with YTD tax M uses the Chapter 5 T formula on the live path.
    #[test]
    fn option2_period_tax_subtracts_ytd_tax_m() {
        let mut req = request_at(Province::On, "2026-01-15", "1000.00");
        req.calculation_option = CalculationOption::Option2;
        req.pay_periods_elapsed = Some(1);
        req.ytd_income = Some(money("1000.00"));
        let without_m = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        req.ytd_federal_tax = Some(money("5000.00"));
        let with_m = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(with_m.breakdown.s1.to_string(), "52/2");
        assert_eq!(with_m.breakdown.m, money("5000.00"));
        // Large M makes T = L = 0.
        assert_eq!(with_m.breakdown.t, money("0.00"));
        assert_ne!(without_m.breakdown.t, with_m.breakdown.t);
    }

    /// Test 24 — prorated-reconciliation warning for BC/NL/PE in H2 2026 only.
    #[test]
    fn prorated_reconciliation_warning_only_for_changed_provinces_in_h2() {
        for province in [Province::Bc, Province::Nl, Province::Pe] {
            let h1 = calculate(
                &request_at(province, "2026-06-30", "1000.00"),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            let h2 = calculate(
                &request_at(province, "2026-07-01", "1000.00"),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            let later = calculate(
                &request_at(province, "2026-12-31", "1000.00"),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            let next_year = calculate(
                &request_at(province, "2027-01-01", "1000.00"),
                &EMBEDDED_REGISTRY,
            )
            .unwrap();
            assert!(!has_prorated_warning(&h1), "{province:?} H1");
            assert!(has_prorated_warning(&h2), "{province:?} 2026-07-01");
            assert!(has_prorated_warning(&later), "{province:?} 2026-12-31");
            assert!(!has_prorated_warning(&next_year), "{province:?} 2027");
        }
        let on_h2 = calculate(
            &request_at(Province::On, "2026-08-01", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert!(!has_prorated_warning(&on_h2));
        let ab_h2 = calculate(
            &request_at(Province::Ab, "2026-08-01", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert!(!has_prorated_warning(&ab_h2));
    }

    fn next_day(d: CalendarDate) -> CalendarDate {
        if let Ok(next) = CalendarDate::new(d.year, d.month, d.day.saturating_add(1)) {
            return next;
        }
        if let Ok(next) = CalendarDate::new(d.year, d.month.saturating_add(1), 1) {
            return next;
        }
        CalendarDate::new(d.year.saturating_add(1), 1, 1).expect("year increment")
    }

    fn add_days(start: CalendarDate, days: u32) -> CalendarDate {
        let mut cur = start;
        for _ in 0..days {
            cur = next_day(cur);
        }
        cur
    }

    /// Test 26 — Alberta K5P floor, just above it, and TCP default 22769.
    #[test]
    fn alberta_k5p_floor_just_above_and_tcp_default() {
        let mut no_td1ab = request_at(Province::Ab, "2026-01-01", "1000.00");
        no_td1ab.provincial_claim_code = None;
        no_td1ab.provincial_tcp = None;
        let floor = calculate(&no_td1ab, &EMBEDDED_REGISTRY).unwrap();
        assert_eq!(floor.breakdown.tcp, money("22769.00"));
        assert_eq!(floor.breakdown.k5p, money("0.00"));
        let credits = floor
            .breakdown
            .k1p
            .checked_add(floor.breakdown.k2p)
            .unwrap();
        assert!(
            credits <= money("4896.00"),
            "typical weekly $1000 is the floor case"
        );

        let mut above = request_at(Province::Ab, "2026-01-01", "1000.00");
        above.provincial_claim_code = None;
        above.provincial_tcp = Some(money("100000.00"));
        let above_resp = calculate(&above, &EMBEDDED_REGISTRY).unwrap();
        let expected = alberta_k5p(above_resp.breakdown.k1p, above_resp.breakdown.k2p).unwrap();
        assert_eq!(above_resp.breakdown.k5p, expected);
        assert!(above_resp.breakdown.k5p > money("0.00"));
    }

    /// Test 27 — BC S from both editions, each capped at T4.
    #[test]
    fn bc_s_both_editions_loaded_and_capped_at_t4() {
        let jan = load_ruleset_2026_01_01().unwrap();
        let july = load_ruleset_2026_07_01().unwrap();
        let jan_bc = jan
            .jurisdictions
            .get(&JurisdictionCode("BC".to_string()))
            .unwrap();
        let july_bc = july
            .jurisdictions
            .get(&JurisdictionCode("BC".to_string()))
            .unwrap();
        let jan_red = jan_bc
            .tax_reduction
            .as_ref()
            .unwrap()
            .get(CalculationOption::Option1);
        assert_eq!(jan_red.basic, money("575.00"));
        assert_eq!(jan_red.dependant, money("41722.00"));
        let july1 = july_bc
            .tax_reduction
            .as_ref()
            .unwrap()
            .get(CalculationOption::Option1);
        assert_eq!(july1.basic, money("805.00"));
        assert_eq!(july1.dependant, money("44952.00"));
        let july2 = july_bc
            .tax_reduction
            .as_ref()
            .unwrap()
            .get(CalculationOption::Option2);
        assert_eq!(july2.basic, money("690.00"));
        assert_eq!(july2.dependant, money("44952.00"));

        for (as_of, option) in [
            ("2026-01-01", CalculationOption::Option1),
            ("2026-07-01", CalculationOption::Option1),
            ("2026-07-01", CalculationOption::Option2),
        ] {
            let mut req = request_at(Province::Bc, as_of, "400.00");
            req.calculation_option = option;
            let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            assert!(
                resp.breakdown.s <= resp.breakdown.t4,
                "{as_of} {option:?}: S {} must be capped at T4 {}",
                resp.breakdown.s,
                resp.breakdown.t4
            );
        }
    }

    /// Test 28 — Yukon K4P uses GrossEmploymentIncome / CEA, same newtype as K4.
    #[test]
    fn yukon_k4p_uses_employment_income_and_cea() {
        let resp = calculate(
            &request_at(Province::Yt, "2026-01-01", "1000.00"),
            &EMBEDDED_REGISTRY,
        )
        .unwrap();
        assert_eq!(resp.breakdown.k4p, money("96.06"));
        assert!(resp.breakdown.k4p > money("0.00"));
    }

    /// Test 29 — LCP variants plus nine jurisdictions with no LCP (None → exactly 0).
    #[test]
    fn lcp_variants_and_none_path_contributes_zero() {
        let set = load_ruleset_2026_01_01().unwrap();
        let with_lcp: &[(&str, &str, &str, Province)] = &[
            ("MB", "0.150", "1800.00", Province::Mb),
            ("NB", "0.200", "2000.00", Province::Nb),
            ("NS", "0.200", "2000.00", Province::Ns),
            ("SK", "0.175", "875.00", Province::Sk),
        ];
        for (code, rate, max, province) in with_lcp {
            let j = set
                .jurisdictions
                .get(&JurisdictionCode((*code).to_string()))
                .unwrap();
            let lcp = j.lcp.as_ref().expect("{code} must have LCP");
            assert_eq!(lcp.rate, Rate::parse(rate).unwrap(), "{code} rate");
            assert_eq!(lcp.max, money(max), "{code} max");
            let mut req = request_at(*province, "2026-01-01", "2000.00");
            req.lcp_purchase = Some(money("10000.00"));
            let with = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            req.lcp_purchase = None;
            let without = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            assert!(
                with.breakdown.lcp > money("0.00"),
                "{code} purchase must credit"
            );
            assert_eq!(without.breakdown.lcp, money("0.00"));
            assert!(
                with.breakdown.t2 < without.breakdown.t2,
                "{code} LCP reduces T2"
            );
        }

        let none: &[(&str, Option<Province>)] = &[
            ("FED", None),
            ("AB", Some(Province::Ab)),
            ("BC", Some(Province::Bc)),
            ("NL", Some(Province::Nl)),
            ("NT", Some(Province::Nt)),
            ("NU", Some(Province::Nu)),
            ("ON", Some(Province::On)),
            ("PE", Some(Province::Pe)),
            ("YT", Some(Province::Yt)),
        ];
        assert_eq!(with_lcp.len() + none.len(), 13);
        for (code, province) in none {
            let j = set
                .jurisdictions
                .get(&JurisdictionCode((*code).to_string()))
                .unwrap();
            assert!(j.lcp.is_none(), "{code} must have no LCP");
            let Some(province) = province else {
                continue;
            };
            let mut req = request_at(*province, "2026-01-01", "1000.00");
            req.lcp_purchase = Some(money("10000.00"));
            let with_purchase = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            req.lcp_purchase = None;
            let without_purchase = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
            assert_eq!(
                with_purchase.breakdown.lcp,
                money("0.00"),
                "{code} None params + purchase → LCP factor 0, not omitted/error"
            );
            assert_eq!(with_purchase.breakdown.t2, without_purchase.breakdown.t2);
            assert_eq!(
                with_purchase.employee.provincial_tax,
                without_purchase.employee.provincial_tax
            );
        }
    }

    /// Test 30 — OutsideCanada full response shape: provincial zeros, 48% surtax, T = (T1/P)+L.
    #[test]
    fn outside_canada_full_response_shape() {
        let mut req = request_at(Province::OutsideCanada, "2026-01-01", "1000.00");
        req.additional_tax_requested = Some(money("10.00"));
        let resp = calculate(&req, &EMBEDDED_REGISTRY).unwrap();
        let b = &resp.breakdown;
        assert_eq!(b.v, Rate::parse("0").unwrap());
        assert_eq!(b.v1, money("0.00"));
        assert_eq!(b.v2, money("0.00"));
        assert_eq!(b.s, money("0.00"));
        assert_eq!(b.lcp, money("0.00"));
        assert_eq!(b.t2, money("0.00"));
        assert_eq!(b.t4, money("0.00"));
        assert_eq!(b.k1p, money("0.00"));
        assert_eq!(b.k2p, money("0.00"));
        assert_eq!(resp.employee.provincial_tax, money("0.00"));
        assert_eq!(resp.annual_projection.provincial_tax, money("0.00"));
        let surtax = b.t3.checked_mul_rate(Rate::parse("0.48").unwrap()).unwrap();
        let expected_t1 = crate::rounding::round_tax_to_cent(
            b.t3.checked_add(surtax)
                .unwrap()
                .checked_div(money("1"))
                .unwrap(),
        );
        assert_eq!(b.t1, expected_t1);
        assert!(b.t1 > b.t3, "48% surtax must raise T1 above T3");
        let expected_t = crate::formulas::per_period::per_period_tax(
            b.t1,
            money("0.00"),
            req.pay_period,
            money("10.00"),
            crate::rounding::Granularity::Cent,
        )
        .unwrap();
        assert_eq!(b.t, expected_t);
        assert_eq!(resp.employee.federal_tax, expected_t);
        assert_eq!(resp.annual_projection.federal_tax, b.t1);
        assert_eq!(fed_surtax_flat(), Rate::parse("0.48").unwrap());
    }

    fn fed_surtax_flat() -> Rate {
        load_ruleset_2026_01_01()
            .unwrap()
            .jurisdictions
            .get(&JurisdictionCode("FED".to_string()))
            .unwrap()
            .surtax_flat
            .expect("FED must carry Table 8.2 Outside Canada surtax")
    }

    /// Test 31 — Quebec is a typed refusal, never a successful federal-only T2 = 0.
    #[test]
    fn quebec_is_jurisdiction_not_supported_not_federal_only() {
        let req = request_at(Province::Qc, "2026-01-01", "1000.00");
        let result = calculate(&req, &EMBEDDED_REGISTRY);
        assert!(
            result.is_err(),
            "Quebec must not succeed; a T2=0 federal-only Ok is the dangerous failure mode"
        );
        match result {
            Err(EngineError::JurisdictionNotSupported {
                jurisdiction,
                reason,
            }) => {
                assert_eq!(jurisdiction, "QC");
                assert!(reason.contains("Quebec"), "{reason}");
                assert!(reason.contains("docs/jurisdictions.md"), "{reason}");
                assert_eq!(reason, QUEBEC_UNSUPPORTED_REASON);
            }
            other => panic!("expected JurisdictionNotSupported, got {other:?}"),
        }
        let listing = list_jurisdictions();
        let qc = listing
            .jurisdictions
            .iter()
            .find(|row| row.code == "QC")
            .unwrap();
        assert!(!qc.supported);
        assert_eq!(qc.name, "Quebec");
        let note = qc.note.as_deref().unwrap();
        assert!(note.contains("Quebec"));
        assert!(note.contains("docs/jurisdictions.md"));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10_000))]
        /// Test 25 — property: every resolved interval contains as_of. 10k dates
        /// from 2025-12-31 to 2027-01-02; outside coverage is DateBefore/After.
        #[test]
        fn resolved_interval_contains_as_of(offset in 0u32..=367) {
            use crate::rules::registry::RuleError;
            let as_of = add_days(date("2025-12-31"), offset);
            prop_assume!(as_of <= date("2027-01-02"));
            match EMBEDDED_REGISTRY.resolve(as_of) {
                Ok(set) => {
                    prop_assert!(as_of >= set.effective_from);
                    if let Some(to) = set.effective_to {
                        prop_assert!(as_of < to);
                    }
                }
                Err(RuleError::DateBeforeCoverage { requested, earliest, .. }) => {
                    prop_assert_eq!(requested, as_of);
                    prop_assert!(as_of < earliest);
                }
                Err(RuleError::DateAfterCoverage { requested, latest, .. }) => {
                    prop_assert_eq!(requested, as_of);
                    prop_assert!(as_of >= latest);
                }
                Err(other) => prop_assert!(false, "unexpected {other}"),
            }
        }
    }
}
