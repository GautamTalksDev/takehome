import { PROVINCES } from './provinces.js';

const FEDERAL = {
  brackets:
    '14% to $58,523; 20.5% to $117,045; 26% to $181,440; 29% to $258,482; 33% above that.',
  bpaf:
    'BPAF is $16,452 below $181,440 and phases to $14,829 by $258,482 (T4127 Chapter 8).',
  cea: 'Canada employment amount K4 uses $1,501.',
  cpp: '2026 YMPE $74,600, YAMPE $85,000, employee 5.95% (4.95% + 1.00%) to YMPE, 4% CPP2 to YAMPE. Max C $4,230.45, C2 $416.00.',
  ei: '2026 MIE $68,900, employee 1.63% (max $1,123.07), employer 1.4× (max $1,572.30).',
};

export const PROVINCE_FACTS = {
  on: {
    t2Line:
      'Ontario T2 includes three extra Chapter 4 pieces no other province has in this combination: the two-step surtax, the Ontario Health Premium, and the Ontario tax reduction.',
    mechanisms: [
      {
        h2: 'Ontario surtax (V1 / V2)',
        p: 'After basic Ontario tax, a 20% surtax applies once Ontario tax exceeds $5,818, and 36% once it exceeds $7,446. That is a surtax on tax, not a sixth income bracket. A biweekly cheque that looks like it is still in the 9.15% band can already be paying surtax because T4 is annual.',
      },
      {
        h2: 'Ontario Health Premium',
        p: 'The OHP is withheld with provincial tax. It is $0 under $20,000 of taxable income, then climbs through $300 / $450 / $600 / $750 caps to $900 above $200,000. It is not a separate payroll line in T4127: it sits inside T2. Employers do not match it.',
      },
      {
        h2: 'Ontario tax reduction',
        p: 'Factor Y: $300 plus $554 per eligible dependant under 19 (or disabled dependant). It is a reduction of Ontario tax, not a credit at the lowest rate, and it phases out as Ontario tax rises. Claim-code 1 alone does not add dependants; that is a separate request field.',
      },
      {
        h2: 'Brackets and BPA, 2026',
        p: '5.05% to $53,891; 9.15% to $107,785; 11.16% to $150,000; 12.16% to $220,000; 13.16% above. TCP is $12,989 (fixed). Lowest rate used for K1P is 5.05%. Ontario did not change tables on 1 July 2026.',
      },
    ],
    employer:
      'Ontario Health Premium and surtax are employee-only. Employer cost in Ontario is still the CPP match (including CPP2) plus 1.4× EI. Do not add OHP to the employer remittance of CPP/EI.',
    cppNote:
      'Ontario uses CPP, not QPP. K2P credits Ontario tax for the provincial share of base CPP and EI at 5.05%.',
    eiNote:
      'Ontario EI is the federal table (1.63% / 1.4×). There is no provincial payroll tax on wages analogous to Quebec’s contribution au Fonds des services de santé in this engine.',
  },
  ab: {
    t2Line:
      'Alberta T2 is the six-rate 2026 table minus K1P, K2P, and the Alberta-only supplemental credit K5P. It is not the old 10% flat tax.',
    mechanisms: [
      {
        h2: 'The 2026 Alberta rates are not flat',
        p: 'T4127 Chapter 4 Alberta for 2026 is 8% to $61,200; 10% to $154,259; 12% to $185,111; 13% to $246,813; 14% to $370,220; 15% above. The first-dollar rate is 8%, not 10%. Anyone still quoting a single Alberta rate is reading a withdrawn year.',
      },
      {
        h2: 'K5P supplemental credit',
        p: 'K5P = max(0, ((K1P + K2P) − $4,896.00) × 0.25), then round to the cent. It is unique to Alberta. It floors at zero: credits at or below $4,896 produce K5P = $0.00. Just above the floor, $0.04 of excess × 0.25 = $0.01. High K1P+K2P (high TCP and/or high CPP/EI credits) increases K5P and lowers T2.',
      },
      {
        h2: 'TCP $22,769',
        p: 'Alberta’s basic personal amount is $22,769 — the highest fixed TCP in the 2026 tables (Yukon’s is federal-linked and can be lower at the top). Combined with the 8% first bracket, low and middle Alberta paycheques have a smaller provincial bite than Ontario or BC at the same gross, before K5P.',
      },
      {
        h2: 'No surtax, no health premium, no tax reduction',
        p: 'Alberta does not run an Ontario-style surtax, OHP, or BC/ON tax reduction. Once you have T4 − K1P − K2P − K5P, you are done with provincial tax. July 2026 inherited the January Alberta file byte-for-byte.',
      },
    ],
    employer:
      'Alberta has no employer health tax in T4127. Employer cost is CPP (matched, including CPP2) plus 1.4× EI. K5P changes the employee’s T2, not the remittance of C or EI.',
    cppNote:
      'Alberta K2P uses the 8% lowest rate × (base CPP + EI). K5P then takes 25% of K1P+K2P above $4,896, so CPP itself feeds the supplemental credit.',
    eiNote:
      'EI is federal. Alberta does not reduce the employee EI rate. The 1.63% / $1,123.07 maximum applies.',
  },
  bc: {
    t2Line:
      'British Columbia T2 is seven brackets, a BC tax reduction, and — on and after 1 July 2026 — Option 1 vs Option 2 first-bracket proration.',
    mechanisms: [
      {
        h2: 'January 2026 first bracket 5.06%',
        p: 'Through 30 June 2026 the first BC rate is 5.06% to $50,363, then 7.70%, 10.50%, 12.29%, 14.70%, 16.80%, 20.50% from $265,545. TCP is $13,216. Indexation used 2.2%.',
      },
      {
        h2: 'July 2026 proration — this is the BC-specific content',
        p: 'The 123rd edition changes the first-bracket rate mid-year. Option 1 (the default payroll method) uses a prorated 6.14% lowest rate and a prorated K constant on that bracket; Option 2 uses 5.60% with different constants. The 7.70% and higher brackets keep their rates; only the first band and the tax reduction amounts are option-scoped. Pay dates on/after 2026-07-01 load rule set 2026-07-01 and the engine sets prorated_rules_applied when Option 1 hits a prorated row.',
      },
      {
        h2: 'BC tax reduction',
        p: 'January: basic $575, with a $41,722 dependant figure in the reduction formula. July Option 1: basic $805; Option 2: $690; dependant $44,952 on both options. This is a reduction of BC tax for lower incomes, not a credit at V. It is why two BC paycheques with the same first-bracket gross can differ between January and July by more than the 5.06% → 6.14% rate change.',
      },
      {
        h2: 'No surtax, no health premium',
        p: 'BC does not add an Ontario-style surtax or OHP. The July change is a first-bracket and reduction change, not a new top rate. Highest statutory rate remains 20.50% from $265,545.',
      },
    ],
    employer:
      'BC employer cost is CPP match + CPP2 match + 1.4× EI. The July first-bracket proration changes employee T2, not C or EI. There is no T4127 employer health tax for BC.',
    cppNote:
      'K2P at BC’s lowest rate: 5.06% before 1 July, 6.14% Option 1 / 5.60% Option 2 from 1 July. That lowest-rate change moves the CPP/EI credit, not the CPP premium itself.',
    eiNote:
      'EI is the federal table. BC does not have a provincial EI rate.',
  },
  mb: {
    t2Line:
      'Manitoba is three brackets and a basic personal amount that phases out between $200,000 and $400,000 — the only province besides the federal/Yukon BPA that shrinks TCP as income rises.',
    mechanisms: [
      {
        h2: 'Three rates',
        p: '10.80% to $47,000; 12.75% to $100,000; 17.40% above. There is no fourth band. The jump at $100,000 is 4.65 points — larger than Ontario’s step at $150,000.',
      },
      {
        h2: 'TCP phase-out',
        p: 'Manitoba BPA is $15,780 at incomes up to $200,000 and phases to $0 at $400,000. That is a Chapter 4 dynamic TCP, not a surtax. A $250,000 Manitoba salary has a smaller personal credit than a $90,000 salary, so T2 rises both from the 17.40% rate and from a shrinking K1P.',
      },
      {
        h2: 'LCP',
        p: 'Labour-sponsored funds credit LCP is 15% to a $1,800 maximum when lcp_purchase is on the request. It is not filled by this calculator unless you pass that field on the wire.',
      },
      {
        h2: 'July 2026',
        p: 'Manitoba was inherited in the 123rd edition. January and July Manitoba JSON are the same tables.',
      },
    ],
    employer:
      'Manitoba’s Health and Post Secondary Education Tax Levy (the payroll tax on the employer) is not a T4127 deduction and is not in this engine. Employer cost here is CPP + CPP2 + 1.4× EI only. Do not use this number as a complete Manitoba employer burden.',
    cppNote:
      'K2P uses 10.80% × (base CPP + EI). The TCP phase-out does not change C.',
    eiNote:
      'Federal EI table. Manitoba does not alter the 1.63% employee rate.',
  },
  sk: {
    t2Line:
      'Saskatchewan is three brackets with a $20,381 TCP — a large personal amount relative to the 10.5% first rate — and an LCP at 17.5%.',
    mechanisms: [
      {
        h2: 'Three rates, high threshold on the middle band',
        p: '10.50% to $54,532; 12.50% to $155,805; 14.50% above. The top Saskatchewan rate is lower than Manitoba’s 17.40% and Ontario’s 13.16% plus surtax. A $180,000 Saskatchewan salary is in the 14.50% band without an extra health premium.',
      },
      {
        h2: 'TCP $20,381',
        p: 'Fixed. Only Alberta’s $22,769 and Nunavut’s $19,659 are in the same neighbourhood among fixed amounts. K1P = 10.50% × $20,381.',
      },
      {
        h2: 'LCP 17.5% / $875',
        p: 'Saskatchewan’s labour-sponsored credit is a higher rate on a smaller maximum than Manitoba or the Maritimes. Unused unless lcp_purchase is supplied.',
      },
      {
        h2: 'July 2026',
        p: 'Inherited. No mid-year Saskatchewan overlay.',
      },
    ],
    employer:
      'Saskatchewan employer cost in this calculator is CPP match + CPP2 + 1.4× EI. Provincial workers’ compensation is outside T4127.',
    cppNote:
      'K2P at 10.50%. CPP premiums themselves are the federal YMPE/YAMPE table.',
    eiNote:
      'Federal EI. No Saskatchewan EI rate.',
  },
  nb: {
    t2Line:
      'New Brunswick is four brackets, TCP $13,664, and an LCP at 20% / $2,000 — a Maritime pattern shared with Nova Scotia, not with Ontario.',
    mechanisms: [
      {
        h2: 'Four rates',
        p: '9.40% to $52,333; 14.00% to $104,666; 16.00% to $193,861; 19.50% above. The first step (9.40% → 14.00%) is 4.6 points, so crossing $52,333 on the annualized A moves T2 more than Ontario’s 5.05% → 9.15% step on a dollar-for-dollar basis.',
      },
      {
        h2: 'No surtax, no health premium',
        p: 'New Brunswick repealed its high-income surtax years ago; it is not in the 2026 file. T4 − K1P − K2P is the provincial tax.',
      },
      {
        h2: 'LCP 20% / $2,000',
        p: 'Same rate and cap as Nova Scotia. Not applied unless lcp_purchase is on the request.',
      },
      {
        h2: 'July 2026',
        p: 'Inherited from January.',
      },
    ],
    employer:
      'Employer cost is CPP + CPP2 + 1.4× EI. New Brunswick has no T4127 employer health premium.',
    cppNote:
      'K2P uses 9.40%.',
    eiNote:
      'Federal EI table.',
  },
  ns: {
    t2Line:
      'Nova Scotia’s first bracket ends at $30,995 — the lowest threshold in the country — so a modest salary is already in the 14.95% band.',
    mechanisms: [
      {
        h2: 'Five rates, early second bracket',
        p: '8.79% to $30,995; 14.95% to $61,991; 16.67% to $97,417; 17.50% to $157,124; 21.00% above. A $45,000 Nova Scotia salary is taxed provincially at 14.95% on the top slice while an Ontario salary at the same gross is still in 5.05%. That is the content of a Nova Scotia take-home page, not a synonym of “Atlantic Canada.”',
      },
      {
        h2: 'TCP $11,932',
        p: 'Among the smaller personal amounts. Combined with the $30,995 threshold, Nova Scotia provincial tax on a $40,000 salary is materially higher than New Brunswick’s 9.40% to $52,333.',
      },
      {
        h2: 'LCP 20% / $2,000',
        p: 'Present in the rule file; applied only with lcp_purchase.',
      },
      {
        h2: 'Indexation 1.6%',
        p: 'Lower than the 2.0% federal factor. Nova Scotia thresholds move slower than Ontario’s.',
      },
    ],
    employer:
      'CPP + CPP2 + 1.4× EI. Nova Scotia has no T4127 employer payroll tax on this page.',
    cppNote:
      'K2P at 8.79% — a lower credit rate than Manitoba, so the same C produces a smaller provincial credit.',
    eiNote:
      'Federal EI.',
  },
  nl: {
    t2Line:
      'Newfoundland and Labrador has eight brackets — the most in T4127 — and a July 2026 TCP jump that is option-scoped.',
    mechanisms: [
      {
        h2: 'Eight rates',
        p: '8.70% to $44,678; 14.50%; 15.80%; 17.80%; 19.80%; 20.80%; 21.30%; 21.80% from $1,141,275. The top two bands exist so that very high A still has a published K. No other province publishes a bracket whose threshold is above $1 million.',
      },
      {
        h2: 'January TCP $11,188; July TCP is option-scoped',
        p: 'The 123rd edition raises the personal amount: Option 1 TCP $15,000, Option 2 $13,094. Brackets and rates are otherwise the same as January. A 30 June vs 1 July NL pay date with identical gross therefore changes K1P, not R. That is a different mid-year event from BC’s first-bracket proration.',
      },
      {
        h2: 'Indexation 1.1%',
        p: 'The lowest provincial index factor in the 2026 files. Thresholds move slowly; the eight-band shape is inherited, not a 2026 invention.',
      },
      {
        h2: 'No LCP in the file',
        p: 'Unlike NS/NB/PE/MB/SK, the NL JSON has no lcp object.',
      },
    ],
    employer:
      'Employer cost is CPP + CPP2 + 1.4× EI. The July TCP change does not change employer premiums.',
    cppNote:
      'K2P at 8.70%. July’s larger TCP increases K1P, not C.',
    eiNote:
      'Federal EI.',
  },
  pe: {
    t2Line:
      'Prince Edward Island added a sixth bracket on 1 July 2026 at $200,000, and Option 1 prorates that new band.',
    mechanisms: [
      {
        h2: 'January: five rates, TCP $15,000',
        p: '9.50% to $33,928; 13.47%; 16.60%; 17.62%; 19.00% from $142,520. No index_rate in the January file. TCP is a round $15,000.',
      },
      {
        h2: 'July: new $200,000 band, Option 1 prorated',
        p: 'Option 1 adds a 21.00% bracket from $200,000 with a prorated K of $10,464. Option 2 uses 20.00% from $200,000 and K $8,464, not marked prorated. Lower five bands are unchanged. This is PE’s analogue of BC’s mid-year change: a new top rate with Option 1 proration, not a first-bracket tweak.',
      },
      {
        h2: 'Early second bracket',
        p: 'The 13.47% band starts at $33,928 — similar to Nova Scotia’s early step, unlike Ontario’s $53,891. A $40,000 PE salary is already in the second provincial rate.',
      },
      {
        h2: 'No LCP, no surtax',
        p: 'PE has neither an LCP object nor a surtax array in 2026.',
      },
    ],
    employer:
      'CPP + CPP2 + 1.4× EI. The July 21% band is employee T2 only.',
    cppNote:
      'K2P at 9.50% both editions.',
    eiNote:
      'Federal EI.',
  },
  nt: {
    t2Line:
      'Northwest Territories is four rates starting at 5.90%, with TCP $18,198 — a northern table without Nunavut’s 4% first band.',
    mechanisms: [
      {
        h2: 'Four rates',
        p: '5.90% to $53,003; 8.60% to $106,009; 12.20% to $172,346; 14.05% above. The top NWT rate (14.05%) is below Alberta’s 15% top and well below BC’s 20.50%.',
      },
      {
        h2: 'TCP $18,198',
        p: 'Fixed, indexed at 2.0%. Higher than Ontario’s $12,989; lower than Alberta’s $22,769.',
      },
      {
        h2: 'Prescribed zone',
        p: 'T4127 factor F1 (prescribed_zone_deduction on the request) reduces A. It is northern-relevant and unused unless supplied. This calculator does not invent a residency deduction.',
      },
      {
        h2: 'July 2026',
        p: 'Inherited. No NWT overlay.',
      },
    ],
    employer:
      'CPP + CPP2 + 1.4× EI. Northern benefits that are taxable are gross, not employer T4127 premiums.',
    cppNote:
      'K2P at 5.90%.',
    eiNote:
      'Federal EI. Insurable earnings still cap at $68,900.',
  },
  nu: {
    t2Line:
      'Nunavut’s first provincial rate is 4.00% — the lowest in T4127 — and TCP is $19,659.',
    mechanisms: [
      {
        h2: 'Four rates, 4% first dollar',
        p: '4.00% to $55,801; 7.00% to $111,602; 9.00% to $181,439; 11.50% above. A $50,000 Nunavut salary has provincial tax that is a fraction of Nova Scotia’s at the same gross. That gap is the page.',
      },
      {
        h2: 'TCP $19,659',
        p: 'Fixed. Combined with 4%, K1P is small in dollars relative to Alberta (8% × $22,769) but large relative to the tax it offsets.',
      },
      {
        h2: 'Top rate 11.50%',
        p: 'The lowest top statutory provincial rate in the 2026 tables. Nunavut does not use an Ontario surtax to claw high incomes.',
      },
      {
        h2: 'Finding 002 density',
        p: 'Fourteen of the 164 one-cent M-003 disagreements are Nunavut. Midpoint direction on T2/P is an open PDOC delta, not a Nunavut-only formula. See /conformance/.',
      },
    ],
    employer:
      'CPP + CPP2 + 1.4× EI. No territorial employer health premium in T4127.',
    cppNote:
      'K2P at 4.00% — the smallest provincial CPP/EI credit rate.',
    eiNote:
      'Federal EI.',
  },
  yt: {
    t2Line:
      'Yukon is the only jurisdiction whose TCP is defined as same_as_federal, including the BPAF phase-out, and whose first three thresholds copy the federal brackets.',
    mechanisms: [
      {
        h2: 'Federal-aligned thresholds, territorial rates',
        p: '6.40% to $58,523; 9.00% to $117,045; 10.90% to $181,440; 12.80% to $500,000; 15.00% above $500,000. The first three cuts are the federal cuts. The 15% territorial top rate starts at $500,000, not at the federal 33% threshold of $258,482.',
      },
      {
        h2: 'TCP = BPAF',
        p: 'type: same_as_federal. Yukon personal amount is $16,452 down to $14,829 as A moves from $181,440 to $258,482. A high Yukon salary loses territorial credit on the same schedule as federal K1. No other province does this.',
      },
      {
        h2: 'Territorial CEA $1,501',
        p: 'Yukon has a Canada employment amount in the provincial file, so K4P is non-zero. Most provinces have no provincial CEA.',
      },
      {
        h2: 'July 2026',
        p: 'Inherited. Eleven of 164 M-003 one-cent rows are Yukon.',
      },
    ],
    employer:
      'CPP + CPP2 + 1.4× EI. Yukon’s federal-linked TCP does not change employer premiums.',
    cppNote:
      'K2P at 6.40%.',
    eiNote:
      'Federal EI.',
  },
  qc: {
    t2Line:
      'Quebec is not T4127 Chapter 4. This engine refuses the jurisdiction rather than return federal tax with T2 = 0.',
    mechanisms: [
      {
        h2: 'Why the calculator is blank',
        p: 'Quebec provincial tax, QPP, QPIP, and the federal abatement (16.5% of basic federal tax) are Revenu Québec / TP-1015.3, not T4127 Chapter 4. Returning a federal-only net with provincial tax zero would look like a number and be wrong. The typed error is JurisdictionNotSupported.',
      },
      {
        h2: 'What still exists federally',
        p: 'Federal brackets, BPAF, CEA, and the 16.5% Quebec abatement are in the federal rule file (abatement 0.165). They are not applied here because we will not emit a partial Quebec paycheque.',
      },
      {
        h2: 'QPP is not CPP',
        p: 'Quebec Pension Plan rates and the additional QPP contributions are not the YMPE/YAMPE CPP table. EI in Quebec is a different employee rate than 1.63% because QPIP exists. None of that is computed on this page.',
      },
      {
        h2: 'Roadmap',
        p: 'docs/jurisdictions.md. Until a Quebec rule set is ingested from Revenu Québec, every Quebec URL on this site is an explanation, not a quiet fallback.',
      },
    ],
    employer:
      'We will not show a Quebec employer cost. QHSF, QPP employer, QPIP employer, and CNESST are outside this crate.',
    cppNote:
      'Quebec employees pay QPP, not CPP. This page will not print C as if the employee were in Ontario.',
    eiNote:
      'Quebec EI employee rate is reduced because QPIP exists. This engine will not print the 1.63% table for a Quebec employment.',
  },
  outsidecanada: {
    t2Line:
      'Outside Canada has no provincial tax. Federal T1 is increased by a 48% surtax, then T2 is zero.',
    mechanisms: [
      {
        h2: '48% federal surtax',
        p: 'T4127 treats employment reported as Outside Canada by applying surtax_flat 0.48 to federal tax. That is not a provincial rate. A $1,000 weekly cheque has a larger federal line than the same cheque in Ontario, and a $0.00 provincial line.',
      },
      {
        h2: 'T2 is always zero',
        p: 'There is no provincial JSON file. K1P, K2P, V, KP are zero. The response still includes those symbols; they are computed-as-zero, not omitted.',
      },
      {
        h2: 'CPP and EI still run',
        p: 'The engine computes C, C2, and EI on the same YMPE/MIE table as a domestic paycheque. Whether a given non-resident employment is actually pensionable or insurable is an HR/CRA facts question this calculator does not decide. It will not drop CPP because the province field says OutsideCanada.',
      },
      {
        h2: 'July 2026',
        p: 'Federal file inherited; surtax_flat remains 0.48. Six of 164 M-003 rows are OutsideCanada.',
      },
    ],
    employer:
      'Employer CPP match + CPP2 + 1.4× EI on whatever pensionable and insurable earnings you enter. No provincial employer tax in T4127 for this code.',
    cppNote:
      'Same 5.95% / 4% CPP2 table. No K2P because T2 is zero; federal K2 still credits T1.',
    eiNote:
      'Federal EI table.',
  },
};

function nameOf(slug) {
  return PROVINCES.find((row) => row.slug === slug)?.name ?? slug;
}

function facts(slug) {
  return PROVINCE_FACTS[slug];
}

export function intentCopy(intent, province) {
  const f = facts(province.slug);
  const name = province.name;
  const commonAfter = f.mechanisms;
  const byId = {
    'take-home-pay': {
      h1: `${name} take-home pay`,
      lede: `Type an ${name} gross, how often you are paid, and a pay date. Takehome subtracts federal tax, provincial tax, CPP, and EI, and shows what you keep this period.`,
      answer:
        'This is T4127 Option 1 net pay for the pay date, frequency, gross, and claim codes you typed. It recalculates in this browser. No salary figure leaves the device.',
    },
    'paycheque-calculator': {
      h1: `${name} paycheque calculator`,
      lede: `This is one ${name} pay stub: gross in, tax, CPP, and EI out. Change a number and the stub updates in this browser.`,
      answer:
        'Change gross or P and the stub lines update. This is a calculation of CRA formulas, not a payroll run, and it does not file a remittance.',
    },
    'salary-calculator': {
      h1: `${name} salary calculator`,
      lede: `Enter a yearly ${name} salary. The result is take-home after a year of the same payroll deductions. Switch the pay frequency to see one cheque.`,
      answer:
        'Option 1 annualizes whatever you put in the gross box: A ≈ P × I. With P = 1, I is the salary. This is not a tax-return projection (no RRSP F, no credits beyond TD1 claim codes).',
    },
    'weekly-pay': {
      h1: `${name} weekly take-home pay`,
      lede: `Weekly take-home in ${name}. Enter this week’s gross; the deductions are the ones that belong on a weekly stub.`,
      answer:
        'Weekly payroll manufactures more rounding events than monthly. Finding 002’s one-cent PDOC disagreements are denser on weekly and daily P. The number is the T4127 half-up result. The CPP basic exemption on a weekly cheque is truncated to the cent: $3,500 / 52 → $67.30, not $67.31.',
    },
    'biweekly-pay': {
      h1: `${name} biweekly take-home pay`,
      lede: `Most Canadian employers pay every two weeks. Enter this period’s ${name} gross to see take-home on that cycle.`,
      answer:
        'Biweekly A = 26 × this period’s I. A 27-period year is a different P (27) — do not reuse this page’s default if your calendar has 27 deposits.',
    },
    'monthly-pay': {
      h1: `${name} monthly take-home pay`,
      lede: `Monthly take-home in ${name}. Enter this month’s gross to see tax, CPP, and EI for a monthly cheque.`,
      answer:
        `Monthly cheques have fewer period-rounding steps, but annual T3/T4 still jump by up to $1.00 at published K thresholds (M-001). Crossing a ${name} bracket on A can move T2 more than the extra dollar of gross. The monthly CPP exemption is $291.66 (truncate $3,500/12), not $291.67.`,
    },
    'payroll-deductions': {
      h1: `${name} payroll deductions`,
      lede: `The amounts to withhold from a ${name} paycheque this period: federal tax, provincial tax, employee CPP, and EI.`,
      answer: f.employer,
    },
    cpp: {
      h1: `${name} CPP calculator`,
      lede: `CPP deducted from a ${name} paycheque this period, and the matching amount the employer remits.`,
      answer: `CPP is federal. ${FEDERAL.cpp} ${f.cppNote} The province changes the K2/K2P credit against tax, not the premium table — except Quebec, which this engine will not calculate. Enter YTD C on a later page (/second-job-payroll-calculator/) when the annual maximum is in play.`,
    },
    ei: {
      h1: `${name} EI calculator`,
      lede: `Employment insurance on a ${name} paycheque this period, and the employer’s 1.4× premium.`,
      answer: `EI is federal. ${FEDERAL.ei} ${f.eiNote} Insurable earnings default to gross. Reduce insurable_earnings on the wire if a portion of the pay is not insurable. Quebec EI is not this table.`,
    },
    'employer-cost': {
      h1: `${name} employer payroll cost`,
      lede: `What it costs to pay a ${name} employee this period: wages plus the employer’s CPP and EI.`,
      answer: `${f.employer} This is not a fully loaded cost: no WCB, no EHT, no RSP match, no vacation accrual. It is the T4127 statutory employer premiums on this period’s pensionable and insurable earnings.`,
    },
  };
  const block = byId[intent.id];
  return {
    title: `${block.h1} — Takehome`,
    h1: block.h1,
    lede: block.lede,
    answer: block.answer,
    t2Line: f.t2Line,
    mechanisms: commonAfter,
    federal: FEDERAL,
    name,
  };
}

export const SITUATION_COPY = {
  'bonus-tax-calculator': {
    h1: 'Bonus tax calculator (T4127)',
    lede: 'A bonus is taxed with a two-pass: tax on annual income including the bonus, minus tax without it. That difference (TB) is withheld this period. PDOC uses the regular method, which is the default here.',
    answer:
      'Regular pay is I; the bonus is B. Chapter 4 computes A with the bonus and A without it, using the same F5A and F5B both times. TB is withheld now, not spread over the remaining periods. If annual taxable income with the bonus is $5,000.00 or less, the engine withholds a flat 15% of the bonus (10% in Quebec) instead of the two-pass.',
    extraLabels: {
      bonus: 'Bonus this period (not included in regular pay)',
    },
  },
  'severance-pay-calculator': {
    h1: 'Severance pay calculator',
    lede: 'If this is ordinary wages paid as a lump, enter it as pay. A true retiring allowance uses different CRA rates; this calculator will not invent those.',
    answer:
      'If the amount is taxable employment income paid on a regular paycheque (salary continuance), enter it as gross — with the lump in the bonus field so F5B splits. If it is a retiring allowance, do not use this number as the withholding; use CRA’s lump-sum rates or PDOC’s retiring-allowance path. We will not pretend those rates are T4127 Chapter 4.',
    extraLabels: {
      bonus: 'Lump paid this period (also included in gross)',
    },
  },
  'retroactive-pay-calculator': {
    h1: 'Retroactive pay calculator',
    lede: 'Back pay is taxed the same way as a bonus: factor B, two-pass TB, withheld on this cheque.',
    answer:
      'Enter regular pay in gross and the retro amount in retroactive_pay. The engine adds it to B with any bonus and runs the Chapter 4 non-periodic path. It does not re-open prior periods.',
    extraLabels: {
      retroactive_pay: 'Retroactive pay this period',
    },
  },
  'vacation-pay-calculator': {
    h1: 'Vacation pay calculator',
    lede: 'Vacation paid as wages is just a larger paycheque. There is no separate vacation tax rate. A 4% or 6% lump on this cheque is more gross, then the usual deductions.',
    answer:
      'Default gross $2,300.00 is an ordinary biweekly $2,000 plus a $300 vacation lump. Change it. If vacation is paid on every cheque as a percent, that percent is already in I — do not add it twice. CPP and EI apply if the amount is pensionable and insurable.',
    extraLabels: {},
  },
  'overtime-pay-calculator': {
    h1: 'Overtime pay calculator',
    lede: 'Overtime is ordinary employment income. Time-and-a-half means a larger cheque, which can push annualized income into a higher tax band.',
    answer:
      'That annualization is why overtime can look “taxed more” on the stub: Option 1 assumes the higher I continues all year. It usually does not. Option 2 (cumulative) is the method that damps that; this page runs Option 1, which is what most payroll software remits.',
    extraLabels: {},
  },
  'commission-tax-calculator': {
    h1: 'Commission tax calculator',
    lede: 'Commission paid this period is entered as gross. If the employee filed a TD1X, put estimated annual expenses in the extra field.',
    answer:
      'Enter this period’s commission draw as gross (Option 1 will annualize it). Put estimated annual expenses in the extra field if the employee filed TD1X. Do not use this page as a full TD1X commission worksheet; it will not replace PDOC’s commission form.',
    extraLabels: {
      estimated_annual_expenses: 'Estimated annual expenses (TD1X)',
    },
  },
  'gross-up-calculator': {
    h1: 'Gross-up calculator',
    lede: 'Type the take-home you need. This page searches for the smallest gross that produces that net, in this browser, against the same engine.',
    answer:
      'Gross-up is not a T4127 formula. It is repeated Option 1 calculations. Ties resolve to the smallest gross (in cents) whose net is at least the target. Claim codes, P, and province all move the answer. Quebec is not available.',
    extraLabels: {
      target_net: 'Target net pay this period',
    },
  },
  'second-job-payroll-calculator': {
    h1: 'Second-job payroll calculator',
    lede: 'A second job usually has no personal amounts (claim code 0) and must count CPP and EI already withheld elsewhere. Enter year-to-date amounts from the other job.',
    answer:
      'Claim code 0 sets TC/TCP to $0. YTD fields reduce remaining room against YMPE, YAMPE, and MIE. This is still Option 1 on this job’s I; it is not a combined T4. If the first job already hit the CPP maximum, C on this cheque should go to $0.00 once YTD C ≥ $4,230.45 (2026, PM = 12).',
    extraLabels: {
      ytd_cpp: 'YTD CPP withheld (all jobs)',
      ytd_cpp2: 'YTD CPP2 withheld (all jobs)',
      ytd_ei: 'YTD EI withheld (all jobs)',
      ytd_pensionable_earnings: 'YTD pensionable earnings (all jobs)',
    },
  },
  'rrsp-payroll-calculator': {
    h1: 'RRSP payroll calculator',
    lede: 'A group RRSP taken from pay is usually after-tax and does not reduce taxable income here. This page will not subtract an RRSP and call it a pension contribution.',
    answer:
      'We will not subtract your RRSP from A and call it F. That would be a silent wrong number. What this page will do: compute T4127 net, and optionally add additional tax requested (L) if you asked the employer to withhold extra toward a tax bill. Union dues (U1) are a different field and are not an RRSP.',
    extraLabels: {
      additional_tax_requested: 'Additional tax requested (L) this period',
    },
  },
  'maternity-top-up-calculator': {
    h1: 'Maternity top-up calculator',
    lede: 'Enter only the top-up (and any wages) paid this period. EI maternity benefits themselves are not a payroll deduction formula.',
    answer:
      'Default $800.00 is a stand-in for a weekly top-up, not an EI benefit. CPP applies if the top-up is pensionable; EI applies if it is insurable. Supplementary unemployment benefits can have different insurable treatment — set insurable earnings on the wire if the SUB plan is not insurable. This page does not calculate EI benefit rates, waiting periods, or RQAP.',
    extraLabels: {},
  },
};

export function extraFieldDefs(situation) {
  const copy = SITUATION_COPY[situation.id];
  const values = {
    bonus: situation.defaultBonus ?? '',
    retroactive_pay: situation.defaultRetro ?? '',
    estimated_annual_expenses: '5000.00',
    target_net: '1500.00',
    ytd_cpp: '0.00',
    ytd_cpp2: '0.00',
    ytd_ei: '0.00',
    ytd_pensionable_earnings: '0.00',
    additional_tax_requested: '0.00',
  };
  return situation.extra.map((name) => ({
    name,
    label: copy.extraLabels[name] ?? name,
    testid: name,
    value: values[name] ?? '',
  }));
}

export function situationCopy(id) {
  return SITUATION_COPY[id];
}

export { FEDERAL, nameOf };
