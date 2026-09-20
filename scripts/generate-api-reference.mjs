#!/usr/bin/env node
/**
 * Generate docs/api-reference.md from services/api/openapi.json.
 * Do not hand-edit api-reference.md; change schema.js and regenerate openapi first.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const OPENAPI = path.join(ROOT, 'services/api/openapi.json');
const OUT = path.join(ROOT, 'docs/api-reference.md');

const ERROR_GUIDANCE = {
  unknown_field: {
    http: 400,
    cause: 'The JSON body includes a property name the engine does not recognize.',
    action: 'Remove or rename the field. See request-fields.md for the allowed set.',
  },
  malformed_json: {
    http: 400,
    cause: 'The body is not valid JSON or exceeds depth or size limits.',
    action: 'Fix syntax. Send application/json. Keep payloads within documented limits.',
  },
  invalid_request: {
    http: 400,
    cause: 'The JSON parsed but violates request rules (shape, enums, or business constraints).',
    action: 'Read error.message. Fix the named constraint and retry.',
  },
  date_out_of_range: {
    http: 400,
    cause: 'as_of falls before the first enacted rule set or after the last covered date.',
    action: 'Pick a supported calendar date. The API does not snap to the nearest rule set.',
  },
  jurisdiction_not_supported: {
    http: 422,
    cause: 'The province code is not calculated (for example QC).',
    action: 'Use GET /v1/jurisdictions. Do not treat a partial total as Quebec payroll tax.',
  },
  rule: {
    http: 400,
    cause: 'The engine rejected rule lookup or diff input.',
    action: 'Confirm rule set versions with GET /v1/rules. Use enacted dates only unless testing proposed sets.',
  },
  engine: {
    http: 500,
    cause: 'An internal engine or health probe failure occurred.',
    action: 'Retry once. If it persists, contact support with rule_set_version and engine_build_sha256 from the envelope.',
  },
  rate_limited: {
    http: 429,
    cause: 'Too many HTTP requests in the rate-limit window for this route or IP.',
    action: 'Backoff and retry. Metering limits are separate; see rate-limits-and-billing.md.',
  },
  not_found: {
    http: 404,
    cause: 'The path, rule version, webhook id, or delivery id does not exist for this account.',
    action: 'Fix the URL or id. Cross-account webhook access returns 404, not 403.',
  },
  method_not_allowed: {
    http: 405,
    cause: 'The HTTP method does not match the route contract.',
    action: 'Use the method shown in this reference for the path.',
  },
  unauthorized: {
    http: 401,
    cause: 'Missing Bearer key, wrong prefix, or revoked secret.',
    action: 'Send Authorization: Bearer np_test_… or np_live_…. Rotate at POST /v1/keys/rotate if lost.',
  },
  plan_limit: {
    http: 402,
    cause: 'The live key reached its UTC-month calculation cap.',
    action: 'Upgrade via POST /v1/billing/checkout or wait until X-Usage-Reset. Failed calls are not billed.',
  },
  batch_too_large: {
    http: 400,
    cause: 'POST /v1/deductions/batch carried more than 1000 requests.',
    action: 'Split the pay run into chunks of at most 1000 calculations.',
  },
  unknown_plan: {
    http: 400,
    cause: 'Checkout plan is not starter, growth, or business.',
    action: 'Send a published plan name from docs/pricing.md.',
  },
  payload_too_large: {
    http: 413,
    cause: 'The request body exceeded the Worker byte limit.',
    action: 'Shrink the batch or payload. Prefer batch for many employees.',
  },
  unsupported_media_type: {
    http: 415,
    cause: 'Content-Type was not application/json where required.',
    action: 'Set content-type: application/json on POST bodies.',
  },
  store: {
    http: 503,
    cause: 'Account, usage, or key storage failed (fail closed).',
    action: 'Retry later. Live metering outages do not serve unmetered calculations.',
  },
};

function refName(ref) {
  if (!ref || typeof ref !== 'string') return null;
  const m = ref.match(/#\/components\/schemas\/(.+)$/);
  return m ? m[1] : null;
}

function schemaType(schema, components, depth = 0) {
  if (!schema || depth > 8) return 'unknown';
  if (schema.$ref) {
    return refName(schema.$ref) ?? 'object';
  }
  if (schema.oneOf) {
    return schema.oneOf.map((s) => schemaType(s, components, depth + 1)).join(' | ');
  }
  if (schema.enum) {
    return schema.enum.map((v) => JSON.stringify(v)).join(' | ');
  }
  if (schema.type) {
    if (Array.isArray(schema.type)) {
      return schema.type.join(' | ');
    }
    return schema.type;
  }
  return 'object';
}

function formatProperties(schema, components, indent = '') {
  if (!schema) return `${indent}(none)\n`;
  if (schema.$ref) {
    return formatProperties(components.schemas[refName(schema.$ref)], components, indent);
  }
  const props = schema.properties ?? {};
  const req = new Set(schema.required ?? []);
  const lines = [];
  for (const [name, prop] of Object.entries(props)) {
    const ty = schemaType(prop, components);
    const reqMark = req.has(name) ? 'required' : 'optional';
    const desc = prop.description ? ` ${prop.description}` : '';
    lines.push(`${indent}- \`${name}\` (${ty}, ${reqMark})${desc}`);
  }
  if (lines.length === 0) {
    return `${indent}(no properties listed)\n`;
  }
  return `${lines.join('\n')}\n`;
}

function collectResponseErrors(responses) {
  const codes = new Set();
  for (const [status, resp] of Object.entries(responses ?? {})) {
    if (status.startsWith('4') || status.startsWith('5')) {
      const content = resp.content?.['application/json'];
      if (content?.schema?.$ref?.includes('ErrorEnvelope')) {
        codes.add(Number(status));
      }
    }
  }
  return [...codes].sort((a, b) => a - b);
}

function main() {
  const doc = JSON.parse(readFileSync(OPENAPI, 'utf8'));
  const components = doc.components ?? {};
  const paths = doc.paths ?? {};
  const version = doc.info?.version ?? '0.1.0';
  const server = doc.servers?.[0]?.url ?? 'https://takehome.gautamkhosla.com';

  const chunks = [];
  chunks.push('# API reference');
  chunks.push('');
  chunks.push('**Who this is for:** Developers integrating the hosted Takehome API who need every route, field, and error code in one place.');
  chunks.push('');
  chunks.push('**When you finish:** You can call any documented route with the correct method, body shape, and typed error handling.');
  chunks.push('');
  chunks.push('Responses follow T4127 (Payroll Deductions Formulas). Year projection responses may cite YMPE (Year\'s Maximum Pensionable Earnings) and CPP2 (second additional CPP) cap fields.');
  chunks.push('');
  chunks.push('Generated from `services/api/openapi.json` (OpenAPI 3.0.3). Regenerate with `node scripts/generate-api-reference.mjs`.');
  chunks.push('');
  chunks.push(`Base URL: ${server}`);
  chunks.push('');
  chunks.push(`API version: ${version}`);
  chunks.push('');
  chunks.push('Authenticated routes expect `Authorization: Bearer np_test_…` or `np_live_…` unless noted.');
  chunks.push('');

  const pathKeys = Object.keys(paths).sort();
  for (const route of pathKeys) {
    const item = paths[route];
    for (const method of Object.keys(item).sort()) {
      const op = item[method];
      if (!op || typeof op !== 'object') continue;
      const title = op.summary ?? op.operationId ?? `${method.toUpperCase()} ${route}`;
      chunks.push(`## ${method.toUpperCase()} ${route}`);
      chunks.push('');
      chunks.push(title);
      chunks.push('');
      if (op.description) {
        chunks.push(op.description);
        chunks.push('');
      }
      if (op.security?.length === 0) {
        chunks.push('Authentication: none.');
        chunks.push('');
      }
      if (op.parameters?.length) {
        chunks.push('### Parameters');
        chunks.push('');
        for (const p of op.parameters) {
          const loc = p.in ?? 'query';
          const req = p.required ? 'required' : 'optional';
          chunks.push(`- \`${p.name}\` (${loc}, ${req}): ${schemaType(p.schema, components)}`);
        }
        chunks.push('');
      }
      const rb = op.requestBody?.content?.['application/json']?.schema;
      if (rb) {
        chunks.push('### Request body');
        chunks.push('');
        const resolved = rb.$ref ? components.schemas[refName(rb.$ref)] : rb;
        if (resolved?.description) {
          chunks.push(resolved.description);
          chunks.push('');
        }
        chunks.push(formatProperties(resolved, components).trimEnd());
        chunks.push('');
      }
      chunks.push('### Responses');
      chunks.push('');
      for (const [status, resp] of Object.entries(op.responses ?? {}).sort()) {
        chunks.push(`#### HTTP ${status}`);
        chunks.push('');
        chunks.push(resp.description ?? '');
        chunks.push('');
        const jsonSchema = resp.content?.['application/json']?.schema;
        const rssSchema = resp.content?.['application/rss+xml'];
        if (jsonSchema) {
          const name = refName(jsonSchema.$ref);
          chunks.push(`Body schema: ${name ?? schemaType(jsonSchema, components)}`);
          chunks.push('');
          if (name && components.schemas[name]) {
            chunks.push(formatProperties(components.schemas[name], components).trimEnd());
            chunks.push('');
          }
        } else if (rssSchema) {
          chunks.push('Body: RSS 2.0 XML string.');
          chunks.push('');
        }
        const headers = resp.headers ?? {};
        for (const [hname, hdef] of Object.entries(headers)) {
          chunks.push(`Header \`${hname}\`: ${hdef.description ?? ''}`);
        }
        if (Object.keys(headers).length) chunks.push('');
      }
      const errStatuses = collectResponseErrors(op.responses);
      if (errStatuses.length) {
        chunks.push('### Error responses on this route');
        chunks.push('');
        chunks.push(`HTTP statuses that return \`ErrorEnvelope\`: ${errStatuses.join(', ')}.`);
        chunks.push('');
      }
    }
  }

  chunks.push('## Error codes');
  chunks.push('');
  chunks.push('Every error JSON matches `ErrorEnvelope`: `error` (`code`, `message`, `docs`) plus `rule_set_version`, `engine_version`, and `engine_build_sha256`.');
  chunks.push('');
  for (const [code, info] of Object.entries(ERROR_GUIDANCE).sort(([a], [b]) => a.localeCompare(b))) {
    chunks.push(`### \`${code}\``);
    chunks.push('');
    chunks.push(`HTTP ${info.http}.`);
    chunks.push('');
    chunks.push(`**Cause:** ${info.cause}`);
    chunks.push('');
    chunks.push(`**What to do:** ${info.action}`);
    chunks.push('');
  }

  chunks.push('**Last reviewed:** 2026-09-20');
  chunks.push('**Engine:** 0.1.0');
  chunks.push('');

  writeFileSync(OUT, chunks.join('\n'), 'utf8');
  console.log(`Wrote ${OUT}`);
}

main();
