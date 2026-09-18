//! Year-projection invariants corpus (API tests 22–29). Oracle is not PDOC.

use crate::{Corpus, Disagreement, ReportError};
use std::collections::BTreeMap;
use takehome_core::request::Request;
use takehome_core::rounding::truncate_exemption_to_cent;
use takehome_core::rules::loader::EMBEDDED_REGISTRY;
use takehome_core::rules::schema::CalendarDate;
use takehome_core::{calculate, Money, Ratio};

struct Period {
    as_of: CalendarDate,
    ytd_pe_before: Money,
    ytd_pe_after: Money,
    cpp: Money,
    cpp2: Money,
    ei: Money,
    provincial_tax: Money,
    exemption: Money,
    p: String,
    rule_set_version: String,
    annual_t1: Money,
}

struct YearWalk {
    periods: Vec<Period>,
    cpp: Money,
    cpp2: Money,
    ei: Money,
    federal_tax: Money,
}

fn money_err(e: impl ToString) -> ReportError {
    ReportError::Message(e.to_string())
}

fn add_money(a: Money, b: Money) -> Result<Money, ReportError> {
    a.checked_add(b).map_err(money_err)
}

fn abs_diff(a: Money, b: Money) -> Result<Money, ReportError> {
    if a >= b {
        a.checked_sub(b).map_err(money_err)
    } else {
        b.checked_sub(a).map_err(money_err)
    }
}

fn next_day(d: CalendarDate) -> Result<CalendarDate, ReportError> {
    if let Ok(next) = CalendarDate::new(d.year, d.month, d.day.saturating_add(1)) {
        return Ok(next);
    }
    if let Ok(next) = CalendarDate::new(d.year, d.month.saturating_add(1), 1) {
        return Ok(next);
    }
    CalendarDate::new(d.year.saturating_add(1), 1, 1).map_err(money_err)
}

fn add_days(start: CalendarDate, days: u32) -> Result<CalendarDate, ReportError> {
    let mut cur = start;
    for _ in 0..days {
        cur = next_day(cur)?;
    }
    Ok(cur)
}

fn interval_days(pay_period: u16) -> Result<u32, ReportError> {
    match pay_period {
        52 | 53 | 240 | 2000 => Ok(7),
        26 | 27 => Ok(14),
        24 => Ok(15),
        13 => Ok(28),
        10 => Ok(36),
        4 => Ok(91),
        2 => Ok(182),
        22 => Ok(16),
        1 => Ok(0),
        other => Err(ReportError::Message(format!(
            "no year-projection interval for P={other}"
        ))),
    }
}

fn pay_dates(first: CalendarDate, pay_period: u16) -> Result<Vec<CalendarDate>, ReportError> {
    let n = usize::from(pay_period);
    let step = interval_days(pay_period)?;
    let mut dates = Vec::with_capacity(n);
    for i in 0..n {
        let offset = u32::try_from(i)
            .map_err(money_err)?
            .checked_mul(step)
            .ok_or_else(|| ReportError::Message("pay date offset overflow".into()))?;
        dates.push(add_days(first, offset)?);
    }
    Ok(dates)
}

fn exemption_for(pay_period: u16) -> Result<Money, ReportError> {
    Ok(truncate_exemption_to_cent(
        Ratio::div("3500.00", &pay_period.to_string()).map_err(money_err)?,
    ))
}

fn walk_year(
    as_of: &str,
    province: &str,
    pay_period: u16,
    gross_pay: &str,
) -> Result<YearWalk, ReportError> {
    let first: CalendarDate = as_of.parse().map_err(money_err)?;
    let dates = pay_dates(first, pay_period)?;
    let exemption = exemption_for(pay_period)?;
    let mut ytd_cpp = Money::ZERO;
    let mut ytd_cpp2 = Money::ZERO;
    let mut ytd_ei = Money::ZERO;
    let mut ytd_pe = Money::ZERO;
    let mut ytd_ie = Money::ZERO;
    let mut total_cpp = Money::ZERO;
    let mut total_cpp2 = Money::ZERO;
    let mut total_ei = Money::ZERO;
    let mut total_fed = Money::ZERO;
    let mut periods = Vec::new();
    let gross = Money::parse(gross_pay).map_err(money_err)?;
    for date in dates {
        let json = format!(
            r#"{{
                "as_of": "{date}",
                "province": "{province}",
                "pay_period": {pay_period},
                "gross_pay": "{gross_pay}",
                "federal_claim_code": 1,
                "provincial_claim_code": 1,
                "ytd_cpp": "{ytd_cpp}",
                "ytd_cpp2": "{ytd_cpp2}",
                "ytd_ei": "{ytd_ei}",
                "ytd_pensionable_earnings": "{ytd_pe}",
                "ytd_insurable_earnings": "{ytd_ie}"
            }}"#
        );
        let req = Request::from_json(&json).map_err(money_err)?;
        let resp = calculate(&req, &EMBEDDED_REGISTRY).map_err(money_err)?;
        let pe_after = add_money(ytd_pe, gross)?;
        let ie_after = add_money(ytd_ie, gross)?;
        let cpp_after = add_money(ytd_cpp, resp.employee.cpp)?;
        let cpp2_after = add_money(ytd_cpp2, resp.employee.cpp2)?;
        let ei_after = add_money(ytd_ei, resp.employee.ei)?;
        total_cpp = add_money(total_cpp, resp.employee.cpp)?;
        total_cpp2 = add_money(total_cpp2, resp.employee.cpp2)?;
        total_ei = add_money(total_ei, resp.employee.ei)?;
        total_fed = add_money(total_fed, resp.employee.federal_tax)?;
        periods.push(Period {
            as_of: date,
            ytd_pe_before: ytd_pe,
            ytd_pe_after: pe_after,
            cpp: resp.employee.cpp,
            cpp2: resp.employee.cpp2,
            ei: resp.employee.ei,
            provincial_tax: resp.employee.provincial_tax,
            exemption,
            p: resp.breakdown.p.to_string(),
            rule_set_version: resp.rule_set_version,
            annual_t1: resp.annual_projection.federal_tax,
        });
        ytd_cpp = cpp_after;
        ytd_cpp2 = cpp2_after;
        ytd_ei = ei_after;
        ytd_pe = pe_after;
        ytd_ie = ie_after;
    }
    Ok(YearWalk {
        periods,
        cpp: total_cpp,
        cpp2: total_cpp2,
        ei: total_ei,
        federal_tax: total_fed,
    })
}

fn cap_period(periods: &[Period], line: fn(&Period) -> Money) -> Option<usize> {
    let mut last_positive = None;
    for (i, period) in periods.iter().enumerate() {
        if line(period) > Money::ZERO {
            last_positive = Some(i);
        }
    }
    let last = last_positive?;
    if last + 1 == periods.len() {
        return None;
    }
    if periods[last + 1..].iter().any(|p| line(p) != Money::ZERO) {
        return None;
    }
    Some(last)
}

fn first_where(periods: &[Period], pred: impl Fn(&Period) -> bool) -> Option<usize> {
    periods.iter().position(pred)
}

fn jan_2026() -> Result<CalendarDate, ReportError> {
    "2026-01-01".parse().map_err(money_err)
}

fn cpp_max() -> Result<Money, ReportError> {
    Ok(EMBEDDED_REGISTRY
        .resolve(jan_2026()?)
        .map_err(money_err)?
        .cpp
        .total_max)
}

fn ei_max() -> Result<Money, ReportError> {
    Ok(EMBEDDED_REGISTRY
        .resolve(jan_2026()?)
        .map_err(money_err)?
        .ei
        .employee_max)
}

fn ympe() -> Result<Money, ReportError> {
    Ok(EMBEDDED_REGISTRY
        .resolve(jan_2026()?)
        .map_err(money_err)?
        .cpp
        .ympe)
}

fn tolerance_p_cents(p: u16) -> Result<Money, ReportError> {
    Money::parse(&format!("0.{p:02}")).map_err(money_err)
}

fn disagreement(
    id: &str,
    request: serde_json::Value,
    engine: BTreeMap<String, String>,
    expected: BTreeMap<String, String>,
    explanation: &str,
) -> Disagreement {
    let mut delta = BTreeMap::new();
    for (k, e) in &expected {
        if engine.get(k) != Some(e) {
            delta.insert(k.clone(), engine.get(k).cloned().unwrap_or_default());
        }
    }
    Disagreement {
        id: id.to_string(),
        request,
        engine,
        pdoc: expected,
        delta,
        explanation: explanation.to_string(),
        pdoc_believed_wrong: None,
    }
}

fn map_one(k: &str, v: impl ToString) -> BTreeMap<String, String> {
    BTreeMap::from([(k.to_string(), v.to_string())])
}

struct Collector {
    matches: u64,
    disagreements: Vec<Disagreement>,
}

impl Collector {
    fn check(
        &mut self,
        id: &str,
        request: serde_json::Value,
        ok: bool,
        engine: BTreeMap<String, String>,
        expected: BTreeMap<String, String>,
        explanation: &str,
    ) {
        if ok {
            self.matches += 1;
        } else {
            self.disagreements
                .push(disagreement(id, request, engine, expected, explanation));
        }
    }
}

/// Eight year-projection invariants (API tests 22–29). Oracle class `invariants`.
pub(crate) fn measure() -> Result<Corpus, ReportError> {
    let mut out = Collector {
        matches: 0,
        disagreements: Vec::new(),
    };
    let defined = 8u64;

    let high = walk_year("2026-01-01", "ON", 26, "4000.00")?;
    let low = walk_year("2026-01-01", "ON", 26, "2000.00")?;
    let bc = walk_year("2026-01-01", "BC", 26, "4000.00")?;
    let weekly53 = walk_year("2026-01-01", "ON", 53, "1000.00")?;
    let biweekly27 = walk_year("2026-01-01", "ON", 27, "2000.00")?;

    let cpp_max = cpp_max()?;
    let ei_max = ei_max()?;
    let ympe = ympe()?;
    let high_req = serde_json::json!({
        "as_of": "2026-01-01",
        "province": "ON",
        "pay_period": 26,
        "gross_pay": "4000.00"
    });

    let cpp_cap = cap_period(&high.periods, |p| p.cpp);
    let cpp_after_zero = cpp_cap
        .map(|i| high.periods[i + 1..].iter().all(|p| p.cpp == Money::ZERO))
        .unwrap_or(false);
    out.check(
        "year-cpp-cap-on-4000-biweekly",
        high_req.clone(),
        high.periods.len() == 26
            && high.cpp == cpp_max
            && cpp_cap.is_some()
            && cpp_after_zero
            && high.periods[cpp_cap.unwrap()].cpp > Money::ZERO,
        map_one("cpp", high.cpp),
        map_one("cpp", cpp_max),
        "API test 22: 26 biweekly periods at $4000.00 sum to CPP total_max and name the cap period",
    );

    let ei_cap = cap_period(&high.periods, |p| p.ei);
    let ei_after_zero = ei_cap
        .map(|i| high.periods[i + 1..].iter().all(|p| p.ei == Money::ZERO))
        .unwrap_or(false);
    out.check(
        "year-ei-cap-on-4000-biweekly",
        high_req.clone(),
        high.ei == ei_max
            && ei_cap.is_some()
            && ei_after_zero
            && high.periods[ei_cap.unwrap()].ei > Money::ZERO,
        map_one("ei", high.ei),
        map_one("ei", ei_max),
        "API test 23: EI sums to the annual maximum and the cap period is named",
    );

    let ympe_cross = first_where(&high.periods, |p| p.ytd_pe_after > ympe);
    let cpp2_start = first_where(&high.periods, |p| p.cpp2 > Money::ZERO);
    let cpp2_before_zero = cpp2_start
        .map(|i| high.periods[..i].iter().all(|p| p.cpp2 == Money::ZERO))
        .unwrap_or(false);
    out.check(
        "year-cpp2-starts-at-ympe-on-4000-biweekly",
        high_req.clone(),
        cpp2_start.is_some()
            && cpp2_start == ympe_cross
            && cpp2_before_zero
            && high.periods[cpp2_start.unwrap()].cpp2 > Money::ZERO,
        map_one("cpp2_start_period", cpp2_start.map(|i| i + 1).unwrap_or(0)),
        map_one("ympe_cross_period", ympe_cross.map(|i| i + 1).unwrap_or(0)),
        "API test 24: CPP2 starts in the YMPE-crossing period and is zero before (spec §21.5)",
    );

    let cross_ok = ympe_cross
        .map(|i| high.periods[i].ytd_pe_before <= ympe && high.periods[i].ytd_pe_after > ympe)
        .unwrap_or(false);
    out.check(
        "year-ympe-cross-on-4000-biweekly",
        high_req,
        ympe_cross.is_some() && cross_ok,
        map_one("ympe_cross_period", ympe_cross.map(|i| i + 1).unwrap_or(0)),
        map_one("ympe", ympe),
        "API test 25: a salary that crosses YMPE mid-year names the crossing period",
    );

    let low_req = serde_json::json!({
        "as_of": "2026-01-01",
        "province": "ON",
        "pay_period": 26,
        "gross_pay": "2000.00"
    });
    let low_cross = first_where(&low.periods, |p| p.ytd_pe_after > ympe);
    let low_cpp2 = first_where(&low.periods, |p| p.cpp2 > Money::ZERO);
    let under_ympe = low.periods.iter().all(|p| p.ytd_pe_after < ympe);
    out.check(
        "year-low-salary-no-cpp2-on-2000-biweekly",
        low_req.clone(),
        low.periods.len() == 26
            && low_cross.is_none()
            && low_cpp2.is_none()
            && low.cpp2 == Money::ZERO
            && under_ympe,
        map_one("cpp2", low.cpp2),
        map_one("cpp2", "0.00"),
        "API test 26: a salary that never reaches YMPE has CPP2 of zero in all 26 periods",
    );

    let june30: CalendarDate = "2026-06-30".parse().map_err(money_err)?;
    let july1: CalendarDate = "2026-07-01".parse().map_err(money_err)?;
    let last_jan = bc
        .periods
        .iter()
        .rev()
        .find(|p| p.rule_set_version == "2026-01-01");
    let first_jul = bc
        .periods
        .iter()
        .find(|p| p.rule_set_version == "2026-07-01");
    let bc_ok = last_jan.is_some()
        && first_jul.is_some()
        && last_jan.unwrap().as_of <= june30
        && first_jul.unwrap().as_of >= july1
        && last_jan.unwrap().provincial_tax != first_jul.unwrap().provincial_tax;
    out.check(
        "year-bc-jan-jul-rule-split",
        serde_json::json!({
            "as_of": "2026-01-01",
            "province": "BC",
            "pay_period": 26,
            "gross_pay": "4000.00"
        }),
        bc_ok,
        map_one(
            "last_jan_as_of",
            last_jan.map(|p| p.as_of.to_string()).unwrap_or_default(),
        ),
        map_one("july_boundary", "2026-07-01"),
        "API test 27: a BC year that crosses 2026-06-30 / 2026-07-01 uses different rules per period",
 );

    let t1 = low.periods[0].annual_t1;
    let gap = abs_diff(low.federal_tax, t1)?;
    let tolerance = tolerance_p_cents(26)?;
    out.check(
        "year-federal-sum-vs-t1-within-p-cents",
        low_req,
        gap <= tolerance,
        map_one("federal_tax_sum", low.federal_tax),
        map_one("annual_t1", t1),
        "API test 28: sum of period federal tax is within one cent per pay period of annual T1",
    );

    let ex53 = exemption_for(53)?;
    let ex27 = exemption_for(27)?;
    let weekly_ok = weekly53.periods.len() == 53
        && weekly53
            .periods
            .iter()
            .all(|p| p.exemption == ex53 && p.p == "53");
    let biweekly_ok = biweekly27.periods.len() == 27
        && biweekly27
            .periods
            .iter()
            .all(|p| p.exemption == ex27 && p.p == "27");
    out.check(
        "year-53-week-and-27-biweekly-exemptions",
        serde_json::json!({
            "P53_exemption": "66.03",
            "P27_exemption": "129.62"
        }),
        weekly_ok && biweekly_ok && ex53.to_string() == "66.03" && ex27.to_string() == "129.62",
        BTreeMap::from([
            ("P53".into(), ex53.to_string()),
            ("P27".into(), ex27.to_string()),
        ]),
        BTreeMap::from([
            ("P53".into(), "66.03".into()),
            ("P27".into(), "129.62".into()),
        ]),
        "API test 29: 53-week and 27-biweekly years produce that many periods with Chapter 6 exemptions",
 );

    Ok(Corpus::from_counts(
        "year-projection-2026",
        "invariants",
        defined,
        defined,
        out.matches,
        out.disagreements,
    ))
}
