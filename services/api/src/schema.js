/**
 * JSON Schema components. This file is the type source for openapi.json.
 * Do not edit openapi.json by hand.
 */

export const DOCS = {
  unknown_field:
    'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  malformed_json:
    'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  invalid_request:
    'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  date_out_of_range: 'https://takehome.gautamkhosla.com/what-is-the-t4127',
  jurisdiction_not_supported: 'https://takehome.gautamkhosla.com/calculators/qc/',
  rule: 'https://takehome.gautamkhosla.com/conformance/',
  engine: 'https://takehome.gautamkhosla.com/what-is-the-t4127',
  rate_limited: 'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  not_found: 'https://takehome.gautamkhosla.com/what-is-the-t4127',
  method_not_allowed:
    'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  unauthorized: 'https://takehome.gautamkhosla.com/signup/',
  plan_limit: 'https://takehome.gautamkhosla.com/pricing/',
  batch_not_implemented: 'https://takehome.gautamkhosla.com/how-payroll-deductions-work-in-canada',
  unknown_plan: 'https://takehome.gautamkhosla.com/pricing/',
};

const money = {
  type: 'string',
  pattern: '^-?\\d+\\.\\d{2}$',
  description: 'Decimal money as a lexical string. Never a JSON number.',
};

const sha = {
  type: 'string',
  pattern: '^[0-9a-f]{64}$',
};

export const schemas = {
  Identity: {
    type: 'object',
    required: ['rule_set_version', 'engine_version', 'engine_build_sha256'],
    properties: {
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string', minLength: 1 },
      engine_build_sha256: sha,
    },
  },
  ErrorBody: {
    type: 'object',
    required: ['code', 'message', 'docs'],
    additionalProperties: false,
    properties: {
      code: { type: 'string', minLength: 1 },
      message: { type: 'string', minLength: 1 },
      docs: { type: 'string', format: 'uri' },
    },
  },
  ErrorEnvelope: {
    type: 'object',
    required: [
      'error',
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
    ],
    properties: {
      error: { $ref: '#/components/schemas/ErrorBody' },
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string', minLength: 1 },
      engine_build_sha256: sha,
    },
  },
  DeductionRequest: {
    type: 'object',
    required: ['province', 'pay_period', 'gross_pay'],
    additionalProperties: false,
    description:
      'T4127 request. Unknown fields are rejected by name. as_of is required by core; if omitted here the Worker sets the UTC request date and does not nearest-match a rule set.',
    properties: {
      as_of: { type: 'string', format: 'date' },
      province: { type: 'string' },
      pay_period: { type: 'integer' },
      gross_pay: money,
      calculation_option: { type: 'string' },
      pensionable_earnings: money,
      insurable_earnings: money,
      federal_claim_code: {},
      provincial_claim_code: {},
      federal_tc: money,
      provincial_tcp: money,
      cpp_months: { type: 'integer' },
      k2_method: { type: 'string' },
      rounding_compat: { type: 'string' },
      ytd_pensionable_earnings: money,
      ytd_insurable_earnings: money,
      ytd_cpp: money,
      ytd_cpp2: money,
      ytd_ei: money,
      ytd_federal_tax: money,
      ytd_provincial_tax: money,
      pay_periods_elapsed: { type: 'integer' },
      bonus: money,
      retroactive_pay: money,
      commission_income: money,
      commission_expenses: money,
      estimated_annual_expenses: money,
      ytd_qpp: money,
      ytd_qpp2: money,
      ytd_qpip: money,
      prior_province: { type: 'string' },
      date_of_birth: { type: 'string', format: 'date' },
      cpp_exempt: { type: 'boolean' },
      cpp_election_after_65: { type: 'boolean' },
      additional_tax_requested: money,
      taxable_benefits: money,
      union_dues: money,
      alimony: money,
      child_care_expenses: money,
      prescribed_zone_deduction: money,
      lcf_purchase: money,
      lcp_purchase: money,
      dependants_under_19: { type: 'integer' },
      dependants_disabled: { type: 'integer' },
    },
  },
  DeductionResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'employee',
      'employer',
    ],
    properties: {
      rule_set_version: { type: 'string' },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      prorated_rules_applied: { type: 'boolean' },
      employee: { type: 'object' },
      employer: { type: 'object' },
      annual_projection: { type: 'object' },
      breakdown: { type: 'object' },
      citations: { type: 'array' },
      warnings: { type: 'array' },
    },
  },
  JurisdictionStatus: {
    type: 'object',
    required: ['code', 'name', 'supported'],
    properties: {
      code: { type: 'string' },
      name: { type: 'string' },
      supported: { type: 'boolean' },
      note: { type: 'string' },
    },
  },
  JurisdictionsResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'jurisdictions',
    ],
    properties: {
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      jurisdictions: {
        type: 'array',
        items: { $ref: '#/components/schemas/JurisdictionStatus' },
      },
    },
  },
  RuleSetVersionStatus: {
    type: 'object',
    required: ['version', 'effective_from'],
    properties: {
      version: { type: 'string' },
      effective_from: { type: 'string' },
      effective_to: { type: ['string', 'null'] },
    },
  },
  RulesListResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'rule_set_versions',
    ],
    properties: {
      rule_set_version: { type: 'string' },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      rule_set_versions: {
        type: 'array',
        items: { $ref: '#/components/schemas/RuleSetVersionStatus' },
      },
    },
  },
  RuleVersionResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'effective_from',
    ],
    properties: {
      rule_set_version: { type: 'string' },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      effective_from: { type: 'string' },
      effective_to: { type: ['string', 'null'] },
    },
  },
  ChangesResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'items',
    ],
    properties: {
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      feed_title: { type: 'string' },
      feed_link: { type: 'string' },
      feed_description: { type: 'string' },
      items: { type: 'array' },
    },
  },
  ConformanceResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'record',
    ],
    properties: {
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      record: { type: 'object' },
    },
  },
  HealthResponse: {
    type: 'object',
    required: [
      'status',
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
    ],
    properties: {
      status: { type: 'string', enum: ['ok'] },
      rule_set_version: { type: 'string' },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
    },
  },
  OpenApiResponse: {
    type: 'object',
    required: [
      'rule_set_version',
      'engine_version',
      'engine_build_sha256',
      'document',
    ],
    properties: {
      rule_set_version: { type: ['string', 'null'] },
      engine_version: { type: 'string' },
      engine_build_sha256: sha,
      document: { type: 'object' },
    },
  },
};

export const moneySchema = money;
