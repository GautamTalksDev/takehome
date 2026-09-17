/**
 * Appendix P — PDOC field locators (captured 2026-09-12 against live PDOC).
 *
 * Element `id` attributes on Angular PDOC are ephemeral UUIDs. Prefer stable
 * `name` attributes when present; otherwise Playwright label / role locators.
 * Do not guess form ids. Update this file only after a fresh capture session.
 */

export const APPENDIX_P_CAPTURED_AT = "2026-09-12";
export const APPENDIX_P_PDOC_IDENTITY = "2026-06-11";

/** Live calendar edition PDOC applies for dates on/after 2026-07-01. */
export const LIVE_RULE_SET_VERSION = "2026-07-01";

export const ENTRY = {
  calculationTypeSalary: {
    name: "calculationType",
    value: "SALARY",
    label: "Salary",
  },
  next: { role: "button" as const, name: "Next" },
};

export const STEP1 = {
  employeeName: { label: /Employee's name/i },
  employerName: { label: /Employer's name/i },
  province: { label: /Province or territory of employment/i },
  payPeriod: { label: /Pay period frequency/i },
  datePaidYear: { id: "datePaidYear", label: /Year portion of the date paid/i },
  datePaidMonth: { label: /^Month$/i },
  datePaidDay: { label: /^Day$/i },
  next: { role: "button" as const, name: "Next" },
};

/** Province select values on PDOC (not Takehome codes). */
export const PROVINCE_VALUE: Record<string, string> = {
  AB: "ALBERTA",
  BC: "BRITISH_COLUMBIA",
  MB: "MANITOBA",
  NB: "NEW_BRUNSWICK",
  NL: "NEWFOUNDLAND_AND_LABRADOR",
  NS: "NOVA_SCOTIA",
  ON: "ONTARIO",
  PE: "PRINCE_EDWARD_ISLAND",
  QC: "QUEBEC",
  SK: "SASKATCHEWAN",
  NT: "NORTHWEST_TERRITORIES",
  NU: "NUNAVUT",
  YT: "YUKON",
  OutsideCanada: "OUTSIDE_CANADA",
};

/**
 * Legal P → PDOC salary pay-period option value.
 * PDOC salary form exposes 10 of 14 legal P values (missing 1, 2, 4, 2000).
 */
export const PAY_PERIOD_VALUE: Record<number, string> = {
  240: "DAILY",
  52: "WEEKLY_52PP",
  26: "BI_WEEKLY",
  24: "SEMI_MONTHLY",
  12: "MONTHLY_12PP",
  10: "TEN_10PP",
  13: "THIRTEEN_13PP",
  22: "TWENTYTWO_22PP",
  53: "WEEKLY_53PP",
  27: "BI_WEEKLY_27PP",
};

export const STEP2 = {
  salary: { label: /Salary or wages income per pay period/i },
  vacationPay: { label: /Vacation pay/i },
  noBonus: {
    name: "salaryType",
    value: "NO_BONUS_PAY_NO_RETROACTIVE_PAY",
    label: /No bonus or retroactive payment/i,
  },
  withBonus: {
    name: "salaryType",
    value: "WITH_BONUS",
    label: /A bonus payment/i,
  },
  next: { role: "button" as const, name: "Next" },
};

/**
 * §11.1 hazard: default is fixed TD1 dollars (`CLAIM_AMOUNT_TD1`).
 * Grid / claim-code cases MUST select claim codes.
 */
export const STEP3 = {
  td1FixedDollars: {
    name: "td1ClaimCodeType",
    value: "CLAIM_AMOUNT_TD1",
    label: /TD1 form/i,
  },
  td1ClaimCodes: {
    name: "td1ClaimCodeType",
    value: "CLAIM_AMOUNT_TD1_CLAIM_CODES",
    label: /Claim codes/i,
  },
  federalClaimCode: {
    label: /federal Form TD1 using claim codes/i,
  },
  provincialClaimCode: {
    label: /provincial or territorial Form TD1 using claim codes/i,
  },
  additionalTax: {
    label: /Requested additional tax deductions from Form TD1/i,
  },
  cppYtd: {
    name: "cppQppType",
    value: "CPP_QPP_YEAR_TO_DATE",
    label: /Year-to-date amount \(from your records\)/i,
  },
  cppMonths: { label: /Number of pensionable months/i },
  pensionableEarningsYtd: {
    label: /Pensionable earnings year-to-date/i,
  },
  cppDeductedYtd: {
    label: /CPP contributions deducted year-to-date/i,
  },
  cpp2DeductedYtd: {
    label: /Second additional CPP contributions deducted year-to-date/i,
  },
  eiYtd: {
    name: "employmentInsuranceType",
    value: "EI_YEAR_TO_DATE",
  },
  insurableEarningsYtd: {
    label: /Insurable earnings year-to-date/i,
  },
  eiDeductedYtd: {
    label: /EI premiums deducted year-to-date/i,
  },
  calculate: { role: "button" as const, name: "Calculate" },
};

export function claimCodeValue(code: number | "E"): string {
  if (code === "E") return "CLAIM_CODE_E";
  return `CLAIM_CODE_${code}`;
}

/** Result-screen line labels → vector expected keys (PDOC English, 2026-09). */
export const RESULTS = {
  federalTax: /Federal tax deduction/i,
  additionalTax: /Additional tax/i,
  provincialTax: /Provincial tax deduction/i,
  cpp: /^CPP deductions$/i,
  cpp2: /^CPP2 deductions$/i,
  ei: /^EI deductions$/i,
  totalDeductions: /^Total deductions$/i,
  netAmount: /^Net amount$/i,
};
