import { PROVINCES } from './provinces.js';

export const EMPLOYEE_INTENTS = [
  {
    id: 'take-home-pay',
    path: 'take-home-pay',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 26,
    gross: '2000.00',
    query: 'take-home pay',
  },
  {
    id: 'paycheque-calculator',
    path: 'paycheque-calculator',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 26,
    gross: '2000.00',
    query: 'paycheque',
  },
  {
    id: 'salary-calculator',
    path: 'salary-calculator',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 1,
    gross: '80000.00',
    query: 'salary',
  },
  {
    id: 'weekly-pay',
    path: 'weekly-pay',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 52,
    gross: '1000.00',
    query: 'weekly pay',
  },
  {
    id: 'biweekly-pay',
    path: 'biweekly-pay',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 26,
    gross: '2000.00',
    query: 'biweekly pay',
  },
  {
    id: 'monthly-pay',
    path: 'monthly-pay',
    kind: 'employee',
    resultMode: 'net',
    payPeriod: 12,
    gross: '4333.33',
    query: 'monthly pay',
  },
];

export const EMPLOYER_INTENTS = [
  {
    id: 'payroll-deductions',
    path: 'payroll-deductions',
    kind: 'employer',
    resultMode: 'deductions',
    payPeriod: 26,
    gross: '2000.00',
    query: 'payroll deductions',
  },
  {
    id: 'cpp',
    path: 'cpp',
    kind: 'employer',
    resultMode: 'cpp',
    payPeriod: 26,
    gross: '2000.00',
    query: 'CPP',
  },
  {
    id: 'ei',
    path: 'ei',
    kind: 'employer',
    resultMode: 'ei',
    payPeriod: 26,
    gross: '2000.00',
    query: 'EI',
  },
  {
    id: 'employer-cost',
    path: 'employer-cost',
    kind: 'employer',
    resultMode: 'employer',
    payPeriod: 26,
    gross: '2000.00',
    query: 'employer payroll cost',
  },
];

export const SITUATIONS = [
  {
    id: 'bonus-tax-calculator',
    extra: ['bonus'],
    defaultBonus: '2500.00',
    defaultGross: '4500.00',
    payPeriod: 26,
  },
  {
    id: 'severance-pay-calculator',
    extra: ['bonus'],
    defaultBonus: '10000.00',
    defaultGross: '12000.00',
    payPeriod: 26,
  },
  {
    id: 'retroactive-pay-calculator',
    extra: ['bonus'],
    defaultBonus: '1500.00',
    defaultGross: '3500.00',
    payPeriod: 26,
  },
  {
    id: 'vacation-pay-calculator',
    extra: [],
    defaultGross: '2300.00',
    payPeriod: 26,
  },
  {
    id: 'overtime-pay-calculator',
    extra: [],
    defaultGross: '2400.00',
    payPeriod: 26,
  },
  {
    id: 'commission-tax-calculator',
    extra: ['estimated_annual_expenses'],
    defaultGross: '3000.00',
    payPeriod: 26,
  },
  {
    id: 'gross-up-calculator',
    extra: ['target_net'],
    defaultGross: '2000.00',
    payPeriod: 26,
    resultMode: 'grossup',
  },
  {
    id: 'second-job-payroll-calculator',
    extra: ['ytd_cpp', 'ytd_cpp2', 'ytd_ei', 'ytd_pensionable_earnings'],
    defaultGross: '800.00',
    payPeriod: 26,
    defaultClaim: '0',
  },
  {
    id: 'rrsp-payroll-calculator',
    extra: ['additional_tax_requested'],
    defaultGross: '2000.00',
    payPeriod: 26,
  },
  {
    id: 'maternity-top-up-calculator',
    extra: [],
    defaultGross: '800.00',
    payPeriod: 26,
  },
];

export const RATE_YEARS = [
  {
    slug: '2026',
    version: '2026-01-01',
    label: 'January 2026',
    edition: '122nd edition',
  },
  {
    slug: '2026-07',
    version: '2026-07-01',
    label: 'July 2026',
    edition: '123rd edition',
  },
];

export const REFERENCE_PAGES = [
  { path: '/changes/', id: 'changes' },
  { path: '/conformance/', id: 'conformance' },
  {
    path: '/how-payroll-deductions-work-in-canada/',
    id: 'how-payroll-deductions-work-in-canada',
  },
  { path: '/what-is-the-t4127/', id: 'what-is-the-t4127' },
  { path: '/cpp-2027-rate-change/', id: 'cpp-2027-rate-change' },
  { path: '/docs/', id: 'docs' },
  { path: '/embed/', id: 'embed' },
  { path: '/pricing/', id: 'pricing' },
  { path: '/signup/', id: 'signup' },
  { path: '/signup/verify/', id: 'signup-verify' },
];

export const INTENTS = [...EMPLOYEE_INTENTS, ...EMPLOYER_INTENTS];

export function intentByPath(path) {
  return INTENTS.find((row) => row.path === path);
}

export function situationById(id) {
  return SITUATIONS.find((row) => row.id === id);
}

export function catalog() {
  const pages = [];
  pages.push({
    path: '/',
    template: 'A',
    hasCalculator: true,
    id: 'home',
  });
  pages.push({
    path: '/calculators/',
    template: 'index',
    hasCalculator: false,
    id: 'calculators-index',
  });
  for (const province of PROVINCES) {
    pages.push({
      path: `/calculators/${province.slug}/`,
      template: 'A',
      hasCalculator: province.supported,
      province,
      id: `calculators-${province.slug}`,
    });
  }
  for (const intent of INTENTS) {
    for (const province of PROVINCES) {
      pages.push({
        path: `/${intent.path}/${province.slug}/`,
        template: intent.kind === 'employee' ? 'A' : 'B',
        hasCalculator: province.supported,
        province,
        intent,
        id: `${intent.id}-${province.slug}`,
      });
    }
  }
  for (const situation of SITUATIONS) {
    pages.push({
      path: `/${situation.id}/`,
      template: 'C',
      hasCalculator: true,
      situation,
      id: situation.id,
    });
  }
  pages.push({
    path: '/rates/',
    template: 'D',
    hasCalculator: false,
    id: 'rates-index',
  });
  for (const year of RATE_YEARS) {
    for (const province of PROVINCES) {
      pages.push({
        path: `/rates/${year.slug}/${province.slug}/`,
        template: 'D',
        hasCalculator: false,
        province,
        year,
        id: `rates-${year.slug}-${province.slug}`,
      });
    }
  }
  for (const ref of REFERENCE_PAGES) {
    pages.push({
      path: ref.path,
      template: 'D',
      hasCalculator: false,
      id: ref.id,
    });
  }
  return pages;
}

export function htmlPaths() {
  return catalog().map((page) => page.path);
}

export function calculatorPaths() {
  return catalog()
    .filter((page) => page.hasCalculator)
    .map((page) => page.path);
}
