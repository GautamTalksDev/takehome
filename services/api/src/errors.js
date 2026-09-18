import { DOCS } from './schema.js';
import { liveIdentity } from './identity.js';

export function corsHeaders() {
  return {
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET, POST, DELETE, OPTIONS',
    'access-control-allow-headers': 'content-type, authorization',
  };
}

export function json(status, body, extra = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      'content-type': 'application/json; charset=utf-8',
      ...corsHeaders(),
      ...extra,
    },
  });
}

export async function errorResponse(
  engine,
  code,
  message,
  status,
  docs,
  extra = {},
) {
  const identity = await liveIdentity(engine, null);
  return json(
    status,
    {
      ...identity,
      error: { code, message, docs },
    },
    extra,
  );
}

export function classifyEngineError(code, message) {
  if (/unknown field/i.test(message)) {
    return {
      code: 'unknown_field',
      status: 400,
      docs: DOCS.unknown_field,
    };
  }
  if (code === 'jurisdiction_not_supported') {
    return {
      code,
      status: 422,
      docs: DOCS.jurisdiction_not_supported,
    };
  }
  if (code === 'rule' && /before coverage|after coverage/.test(message)) {
    return {
      code: 'date_out_of_range',
      status: 400,
      docs: DOCS.date_out_of_range,
    };
  }
  const status =
    code === 'engine' ? 500 : code === 'jurisdiction_not_supported' ? 422 : 400;
  return {
    code,
    status,
    docs: DOCS[code] ?? DOCS.invalid_request,
  };
}

export async function fromEngineJson(raw, engine, extra = {}) {
  const parsed = JSON.parse(raw);
  if (parsed.error) {
    const mapped = classifyEngineError(
      parsed.error.code,
      parsed.error.message,
    );
    return errorResponse(
      engine,
      mapped.code,
      parsed.error.message,
      mapped.status,
      mapped.docs,
      extra,
    );
  }
  return json(200, parsed, extra);
}
