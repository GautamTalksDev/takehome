import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';
import { buildOpenApi } from '../src/openapi.js';
import { call, ON_WEEKLY } from './helpers.js';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const OPENAPI_PATH = path.join(ROOT, '..', 'openapi.json');

function validatorFor(spec) {
  const ajv = new Ajv({ strict: false, allErrors: true });
  addFormats(ajv);
  for (const [name, schema] of Object.entries(spec.components.schemas)) {
    ajv.addSchema(schema, `#/components/schemas/${name}`);
  }
  return ajv;
}

function schemaRef(spec, pathName, method, status) {
  return spec.paths[pathName][method].responses[String(status)].content[
    'application/json'
  ].schema;
}

test('18. openapi.json is generated from src/schema.js, not hand-edited', () => {
  const generated = buildOpenApi();
  const onDisk = JSON.parse(readFileSync(OPENAPI_PATH, 'utf8'));
  assert.deepEqual(onDisk, generated);
});

test('18. Worker responses validate against openapi.json', async () => {
  const spec = JSON.parse(readFileSync(OPENAPI_PATH, 'utf8'));
  const ajv = validatorFor(spec);

  const cases = [
    {
      path: '/v1/deductions',
      method: 'post',
      call: () => call('POST', '/v1/deductions', ON_WEEKLY),
      status: 200,
    },
    {
      path: '/v1/deductions',
      method: 'post',
      call: () =>
        call('POST', '/v1/deductions', { ...ON_WEEKLY, typo_gross: '1.00' }),
      status: 400,
    },
    {
      path: '/v1/deductions',
      method: 'post',
      call: () =>
        call('POST', '/v1/deductions', { ...ON_WEEKLY, province: 'QC' }),
      status: 422,
    },
    {
      path: '/v1/deductions/batch',
      method: 'post',
      call: () =>
        call('POST', '/v1/deductions/batch', { requests: [ON_WEEKLY] }),
      status: 200,
    },
    {
      path: '/v1/deductions/year',
      method: 'post',
      call: () =>
        call('POST', '/v1/deductions/year', {
          ...ON_WEEKLY,
          as_of: '2026-01-01',
          pay_period: 26,
          gross_pay: '2000.00',
        }),
      status: 200,
    },
    {
      path: '/v1/rules',
      method: 'get',
      call: () => call('GET', '/v1/rules'),
      status: 200,
    },
    {
      path: '/v1/rules/diff',
      method: 'get',
      call: () => call('GET', '/v1/rules/diff?from=2026-01-01&to=2026-07-01'),
      status: 200,
    },
    {
      path: '/v1/rules/{version}',
      method: 'get',
      call: () => call('GET', '/v1/rules/2026-07-01'),
      status: 200,
    },
    {
      path: '/v1/jurisdictions',
      method: 'get',
      call: () => call('GET', '/v1/jurisdictions'),
      status: 200,
    },
    {
      path: '/v1/changes',
      method: 'get',
      call: () => call('GET', '/v1/changes'),
      status: 200,
    },
    {
      path: '/v1/conformance',
      method: 'get',
      call: () => call('GET', '/v1/conformance'),
      status: 200,
    },
    {
      path: '/health',
      method: 'get',
      call: () => call('GET', '/health'),
      status: 200,
    },
    {
      path: '/openapi.json',
      method: 'get',
      call: () => call('GET', '/openapi.json'),
      status: 200,
    },
  ];

  for (const row of cases) {
    const { status, json } = await row.call();
    assert.equal(status, row.status, row.path);
    const schema = schemaRef(spec, row.path, row.method, row.status);
    const validate = ajv.compile(schema);
    const ok = validate(json);
    assert.ok(
      ok,
      `${row.method.toUpperCase()} ${row.path} ${row.status}: ${ajv.errorsText(validate.errors)}`,
    );
  }
});
