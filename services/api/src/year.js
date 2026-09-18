/**
 * Full-year T4127 projection: one calculate per pay period with YTD
 * CPP / CPP2 / EI / pensionable / insurable earnings carried forward.
 *
 * CPP basic exemption is T4127 Chapter 6 truncate(3500 / P) at PM = 12.
 */

import { addMoney, centsToMoney, cmpMoney } from './money.js';

const ZERO = '0.00';
const BASIC_EXEMPTION_CENTS = 350000;
const YMPE_2026 = '74600.00';

export function cppBasicExemption(payPeriod, cppMonths = 12) {
  const p = Number.parseInt(String(payPeriod), 10);
  const pm = Number.parseInt(String(cppMonths), 10);
  const numerator = BASIC_EXEMPTION_CENTS * pm;
  const denominator = 12 * p;
  return centsToMoney(Math.trunc(numerator / denominator));
}

export function payIntervalDays(payPeriod) {
  switch (payPeriod) {
    case 52:
    case 53:
    case 240:
    case 2000:
      return 7;
    case 26:
    case 27:
      return 14;
    case 24:
      return 15;
    case 13:
      return 28;
    case 12:
      return 30;
    case 10:
      return 36;
    case 4:
      return 91;
    case 2:
      return 182;
    case 1:
      return 0;
    case 22:
      return 16;
    default:
      return null;
  }
}

export function addDays(iso, days) {
  const [year, month, day] = iso.split('-').map((part) => Number.parseInt(part, 10));
  const utc = Date.UTC(year, month - 1, day + days);
  const date = new Date(utc);
  const y = String(date.getUTCFullYear());
  const m = String(date.getUTCMonth() + 1).padStart(2, '0');
  const d = String(date.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

export function payDates(firstIso, payPeriod) {
  const n = Number.parseInt(String(payPeriod), 10);
  if (n === 12) {
    const dates = [];
    const [year, month, day] = firstIso.split('-').map((part) => Number.parseInt(part, 10));
    for (let i = 0; i < 12; i += 1) {
      const utc = Date.UTC(year, month - 1 + i, day);
      const date = new Date(utc);
      const y = String(date.getUTCFullYear());
      const m = String(date.getUTCMonth() + 1).padStart(2, '0');
      const d = String(date.getUTCDate()).padStart(2, '0');
      dates.push(`${y}-${m}-${d}`);
    }
    return dates;
  }
  if (n === 1) {
    return [firstIso];
  }
  const step = payIntervalDays(n);
  if (step == null) {
    return null;
  }
  const dates = [];
  for (let i = 0; i < n; i += 1) {
    dates.push(addDays(firstIso, i * step));
  }
  return dates;
}

function firstWhere(periods, pred) {
  const index = periods.findIndex(pred);
  return index < 0 ? null : index + 1;
}

/**
 * Identify the period where the annual maximum binds: the last period with a
 * positive contribution after which every later period is 0.00. If every
 * remaining period still withholds, the year never hit the cap.
 */
export function contributionCapPeriod(periods, line) {
  let lastPositive = -1;
  for (let i = 0; i < periods.length; i += 1) {
    if (cmpMoney(periods[i].response.employee[line], ZERO) > 0) {
      lastPositive = i;
    }
  }
  if (lastPositive < 0) {
    return null;
  }
  if (lastPositive === periods.length - 1) {
    return null;
  }
  for (let i = lastPositive + 1; i < periods.length; i += 1) {
    if (periods[i].response.employee[line] !== ZERO) {
      return null;
    }
  }
  return lastPositive + 1;
}

export async function projectYear(payload, engine) {
  const payPeriod = payload.pay_period;
  const dates = payDates(payload.as_of, payPeriod);
  if (!dates) {
    return {
      error: {
        code: 'invalid_request',
        message: `Cannot project a year for pay_period ${payPeriod}.`,
      },
    };
  }
  const cppMonths = payload.cpp_months ?? 12;
  const exemption = cppBasicExemption(payPeriod, cppMonths);
  const gross = payload.gross_pay;
  const pensionable = payload.pensionable_earnings ?? gross;
  const insurable = payload.insurable_earnings ?? gross;

  let ytdCpp = ZERO;
  let ytdCpp2 = ZERO;
  let ytdEi = ZERO;
  let ytdPe = ZERO;
  let ytdIe = ZERO;
  const periods = [];

  for (let i = 0; i < dates.length; i += 1) {
    const request = {
      as_of: dates[i],
      province: payload.province,
      pay_period: payPeriod,
      gross_pay: gross,
      federal_claim_code: payload.federal_claim_code ?? 1,
      provincial_claim_code: payload.provincial_claim_code ?? 1,
      cpp_months: cppMonths,
      ytd_cpp: ytdCpp,
      ytd_cpp2: ytdCpp2,
      ytd_ei: ytdEi,
      ytd_pensionable_earnings: ytdPe,
      ytd_insurable_earnings: ytdIe,
    };
    if (payload.pensionable_earnings != null) {
      request.pensionable_earnings = payload.pensionable_earnings;
    }
    if (payload.insurable_earnings != null) {
      request.insurable_earnings = payload.insurable_earnings;
    }
    if (payload.cpp_exempt != null) {
      request.cpp_exempt = payload.cpp_exempt;
    }
    const parsed = JSON.parse(
      await Promise.resolve(engine.calculate(JSON.stringify(request))),
    );
    if (parsed.error) {
      return { error: parsed.error };
    }
    const employee = parsed.employee;
    const peAfter = addMoney(ytdPe, pensionable);
    const ieAfter = addMoney(ytdIe, insurable);
    const cppAfter = addMoney(ytdCpp, employee.cpp);
    const cpp2After = addMoney(ytdCpp2, employee.cpp2);
    const eiAfter = addMoney(ytdEi, employee.ei);
    periods.push({
      period: i + 1,
      as_of: dates[i],
      cpp_basic_exemption: exemption,
      ytd_pensionable_earnings_before: ytdPe,
      ytd_pensionable_earnings_after: peAfter,
      ytd_cpp_after: cppAfter,
      ytd_ei_after: eiAfter,
      response: parsed,
    });
    ytdCpp = cppAfter;
    ytdCpp2 = cpp2After;
    ytdEi = eiAfter;
    ytdPe = peAfter;
    ytdIe = ieAfter;
  }

  const ympeCrossPeriod = firstWhere(
    periods,
    (period) => cmpMoney(period.ytd_pensionable_earnings_after, YMPE_2026) > 0,
  );
  const cpp2StartPeriod = firstWhere(
    periods,
    (period) => cmpMoney(period.response.employee.cpp2, ZERO) > 0,
  );

  let federal = ZERO;
  let cpp = ZERO;
  let cpp2 = ZERO;
  let ei = ZERO;
  for (const period of periods) {
    federal = addMoney(federal, period.response.employee.federal_tax);
    cpp = addMoney(cpp, period.response.employee.cpp);
    cpp2 = addMoney(cpp2, period.response.employee.cpp2);
    ei = addMoney(ei, period.response.employee.ei);
  }

  return {
    periods,
    totals: {
      federal_tax: federal,
      cpp,
      cpp2,
      ei,
    },
    annual_t1: periods[0].response.annual_projection.federal_tax,
    federal_tax_sum_tolerance: centsToMoney(dates.length),
    cpp_cap_period: contributionCapPeriod(periods, 'cpp'),
    ei_cap_period: contributionCapPeriod(periods, 'ei'),
    cpp2_start_period: cpp2StartPeriod,
    ympe_cross_period: ympeCrossPeriod,
    ympe: YMPE_2026,
  };
}
