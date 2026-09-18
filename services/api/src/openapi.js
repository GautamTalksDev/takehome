import { schemas } from './schema.js';

export function buildOpenApi() {
  const ref = (name) => ({ $ref: `#/components/schemas/${name}` });
  const json = (schemaName) => ({
    content: {
      'application/json': { schema: ref(schemaName) },
    },
  });
  const usageHeaders = {
    'X-Usage-Limit': {
      schema: { type: 'string' },
      description:
        'Monthly live calculation cap, or unlimited for np_test_ keys.',
    },
    'X-Usage-Remaining': {
      schema: { type: 'string' },
      description: 'Calculations remaining this UTC month, or unlimited.',
    },
    'X-Usage-Reset': {
      schema: { type: 'string' },
      description: 'Next UTC month start, when the live meter resets.',
    },
  };

  return {
    openapi: '3.0.3',
    info: {
      title: 'Takehome API',
      version: '0.1.0',
      description:
        'T4127 payroll deductions. Generated from services/api/src/schema.js. Do not hand-edit openapi.json.',
    },
    servers: [{ url: 'https://takehome.gautamkhosla.com' }],
    security: [{ bearerAuth: [] }],
    paths: {
      '/v1/deductions': {
        post: {
          operationId: 'createDeduction',
          summary: 'Calculate T4127 payroll deductions for one pay period.',
          description:
            'Requires a Bearer np_test_ or np_live_ key. Metering counts calculations, not HTTP requests. A batch of 1000 on POST /v1/deductions/batch counts 1000. Test keys are free and unmetered. Live keys hard-stop at the plan limit; Takehome does not surprise-bill.',
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: ref('DeductionRequest') },
            },
          },
          responses: {
            200: {
              description: 'Deduction',
              headers: usageHeaders,
              ...json('DeductionResponse'),
            },
            400: {
              description: 'Rejected request',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            402: {
              description: 'Plan calculation limit reached',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            422: {
              description: 'Jurisdiction not supported',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/deductions/batch': {
        post: {
          operationId: 'createDeductionBatch',
          summary: 'Calculate T4127 payroll deductions for a pay run of up to 1000 employees.',
          description:
            'Requires a Bearer np_test_ or np_live_ key. One HTTP call, up to 1000 calculations, metered as N not 1. Each result is {ok, response} or {error}; one bad employee record does not abort the others. If the live plan cannot cover the whole batch, the call is 402 and usage is unchanged.',
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: ref('BatchRequest') },
            },
          },
          responses: {
            200: {
              description: 'Pay run results, in input order',
              headers: usageHeaders,
              ...json('BatchResponse'),
            },
            400: {
              description: 'Rejected request, including batches over 1000',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            402: {
              description: 'Plan calculation limit reached; nothing billed',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/deductions/year': {
        post: {
          operationId: 'createDeductionYear',
          summary: 'Project every pay period in a year with YTD CPP, CPP2, and EI carried forward.',
          description:
            'Requires a Bearer np_test_ or np_live_ key. Metered as P calculations, not 1. Identifies the CPP cap period, EI cap period, YMPE crossing, and CPP2 start (spec §21.5). Sum of per-period federal tax versus annual T1 is allowed to differ by one cent per pay period because of per-period rounding.',
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: ref('DeductionRequest') },
            },
          },
          responses: {
            200: {
              description: 'Year projection',
              headers: usageHeaders,
              ...json('YearResponse'),
            },
            400: {
              description: 'Rejected request',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            402: {
              description: 'Plan calculation limit reached; nothing billed',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            422: {
              description: 'Jurisdiction not supported',
              headers: usageHeaders,
              ...json('ErrorEnvelope'),
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/signup': {
        post: {
          operationId: 'signup',
          summary: 'Start signup. Email, verify, keys. No sales call.',
          security: [],
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: ref('SignupRequest') },
            },
          },
          responses: {
            200: { description: 'Verification email sent', ...json('SignupResponse') },
            400: { description: 'Rejected request', ...json('ErrorEnvelope') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/signup/verify': {
        get: {
          operationId: 'verifySignup',
          summary: 'Verify email and issue np_test_ plus np_live_ keys.',
          security: [],
          parameters: [
            {
              name: 'token',
              in: 'query',
              required: true,
              schema: { type: 'string' },
            },
          ],
          responses: {
            200: { description: 'Keys issued once', ...json('VerifyResponse') },
            400: { description: 'Invalid token', ...json('ErrorEnvelope') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/billing/checkout': {
        post: {
          operationId: 'createCheckout',
          summary: 'Stripe Checkout session in CAD for a published self-serve tier.',
          requestBody: {
            required: true,
            content: {
              'application/json': { schema: ref('CheckoutRequest') },
            },
          },
          responses: {
            200: { description: 'Checkout URL', ...json('CheckoutResponse') },
            400: { description: 'Unknown plan', ...json('ErrorEnvelope') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/billing/webhook': {
        post: {
          operationId: 'stripeWebhook',
          summary: 'Stripe webhook. Upgrades the plan. Never invents overage.',
          security: [],
          responses: {
            200: { description: 'Received', ...json('WebhookResponse') },
            400: { description: 'Invalid event', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/webhooks': {
        get: {
          operationId: 'listWebhooks',
          summary: 'Webhook endpoints for this account.',
          responses: {
            200: { description: 'Endpoints', ...json('WebhookListResponse') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
          },
        },
        post: {
          operationId: 'createWebhook',
          summary: 'Register an HTTPS endpoint. Signing secret is shown once.',
          requestBody: {
            required: true,
            content: {
              'application/json': {
                schema: {
                  type: 'object',
                  required: ['url'],
                  properties: { url: { type: 'string', format: 'uri' } },
                },
              },
            },
          },
          responses: {
            200: { description: 'Created', ...json('WebhookCreateResponse') },
            400: { description: 'Rejected request', ...json('ErrorEnvelope') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/webhooks/dispatch': {
        post: {
          operationId: 'dispatchWebhooks',
          summary: 'Fan out a rule_set.changed event to every endpoint on this account.',
          requestBody: {
            required: true,
            content: {
              'application/json': {
                schema: {
                  type: 'object',
                  required: ['from', 'to'],
                  properties: {
                    from: { type: 'string' },
                    to: { type: 'string' },
                  },
                },
              },
            },
          },
          responses: {
            200: { description: 'Deliveries', ...json('WebhookDispatchResponse') },
            400: { description: 'Rejected request', ...json('ErrorEnvelope') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            404: { description: 'Unknown rule set', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/webhooks/deliveries': {
        get: {
          operationId: 'listWebhookDeliveries',
          summary: 'Delivery log for this account.',
          responses: {
            200: { description: 'Log', ...json('WebhookDeliveriesResponse') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/webhooks/deliveries/{id}/replay': {
        post: {
          operationId: 'replayWebhookDelivery',
          summary: 'Re-send one signed payload.',
          parameters: [
            {
              name: 'id',
              in: 'path',
              required: true,
              schema: { type: 'string' },
            },
          ],
          responses: {
            200: { description: 'Replayed', ...json('WebhookReplayResponse') },
            401: { description: 'Missing or unknown key', ...json('ErrorEnvelope') },
            404: { description: 'Unknown delivery', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/rules': {
        get: {
          operationId: 'listRules',
          summary: 'Embedded T4127 rule-set editions.',
          security: [],
          responses: {
            200: { description: 'Rule set list', ...json('RulesListResponse') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/rules/diff': {
        get: {
          operationId: 'diffRules',
          summary: 'Field-by-field comparison of two embedded T4127 editions.',
          security: [],
          parameters: [
            {
              name: 'from',
              in: 'query',
              required: true,
              schema: { type: 'string' },
            },
            {
              name: 'to',
              in: 'query',
              required: true,
              schema: { type: 'string' },
            },
          ],
          responses: {
            200: { description: 'Rule-set diff', ...json('RulesDiffResponse') },
            400: { description: 'Missing versions', ...json('ErrorEnvelope') },
            404: { description: 'Unknown version', ...json('ErrorEnvelope') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/rules/{version}': {
        get: {
          operationId: 'getRule',
          summary: 'One embedded rule-set edition. Unknown versions 404; never nearest-match.',
          security: [],
          parameters: [
            {
              name: 'version',
              in: 'path',
              required: true,
              schema: { type: 'string' },
            },
          ],
          responses: {
            200: { description: 'Rule set', ...json('RuleVersionResponse') },
            404: { description: 'Unknown version', ...json('ErrorEnvelope') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/jurisdictions': {
        get: {
          operationId: 'listJurisdictions',
          summary: 'Provinces and territories. Quebec is listed as unsupported.',
          security: [],
          responses: {
            200: {
              description: 'Jurisdictions',
              ...json('JurisdictionsResponse'),
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/changes': {
        get: {
          operationId: 'listChanges',
          summary: 'Machine-readable change log.',
          security: [],
          responses: {
            200: { description: 'Changes', ...json('ChangesResponse') },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/changes.rss': {
        get: {
          operationId: 'listChangesRss',
          summary: 'RSS 2.0 change log.',
          security: [],
          responses: {
            200: {
              description: 'RSS',
              content: {
                'application/rss+xml': { schema: { type: 'string' } },
              },
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/v1/conformance': {
        get: {
          operationId: 'getConformance',
          summary: 'PDOC conformance record, including every disagreement.',
          security: [],
          responses: {
            200: {
              description: 'Conformance',
              ...json('ConformanceResponse'),
            },
            429: { description: 'Rate limited', ...json('ErrorEnvelope') },
          },
        },
      },
      '/health': {
        get: {
          operationId: 'health',
          summary: 'Liveness plus a probe calculation.',
          security: [],
          responses: {
            200: { description: 'OK', ...json('HealthResponse') },
            503: { description: 'Engine failed the probe', ...json('ErrorEnvelope') },
          },
        },
      },
      '/openapi.json': {
        get: {
          operationId: 'getOpenApi',
          summary: 'OpenAPI document generated from the same schemas as this service.',
          security: [],
          responses: {
            200: { description: 'OpenAPI envelope', ...json('OpenApiResponse') },
          },
        },
      },
    },
    components: {
      securitySchemes: {
        bearerAuth: {
          type: 'http',
          scheme: 'bearer',
          description:
            'np_test_ or np_live_ key. Stored hashed. Test keys run the same engine, unmetered.',
        },
      },
      schemas: {
        ...schemas,
        SignupRequest: {
          type: 'object',
          required: ['email'],
          additionalProperties: false,
          properties: {
            email: { type: 'string', format: 'email' },
          },
        },
        SignupResponse: {
          type: 'object',
          required: ['message'],
          properties: {
            message: { type: 'string' },
          },
        },
        VerifyResponse: {
          type: 'object',
          required: ['test_key', 'live_key', 'message'],
          properties: {
            test_key: { type: 'string', pattern: '^np_test_[0-9a-f]{48}$' },
            live_key: { type: 'string', pattern: '^np_live_[0-9a-f]{48}$' },
            message: { type: 'string' },
          },
        },
        CheckoutRequest: {
          type: 'object',
          required: ['plan'],
          properties: {
            plan: { type: 'string', enum: ['starter', 'growth', 'business'] },
            success_url: { type: 'string', format: 'uri' },
            cancel_url: { type: 'string', format: 'uri' },
          },
        },
        CheckoutResponse: {
          type: 'object',
          required: ['url', 'currency', 'plan'],
          properties: {
            url: { type: 'string', format: 'uri' },
            currency: { type: 'string', enum: ['cad'] },
            plan: { type: 'string' },
          },
        },
        WebhookResponse: {
          type: 'object',
          required: ['received'],
          properties: {
            received: { type: 'boolean' },
          },
        },
      },
    },
  };
}
