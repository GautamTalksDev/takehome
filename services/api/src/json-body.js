/**
 * A10:2025 item 53. JSON request bodies are size-capped, depth-capped,
 * and rejected when objects contain duplicate keys. Fail closed: a body
 * we cannot safely parse never reaches the engine.
 */

export const MAX_BODY_BYTES = 1_048_576;
export const MAX_JSON_DEPTH = 16;

export class BodyError extends Error {
  constructor(code, message, status) {
    super(message);
    this.name = 'BodyError';
    this.code = code;
    this.status = status;
  }
}

export function isJsonContentType(value) {
  const media = String(value ?? '')
    .split(';')[0]
    .trim()
    .toLowerCase();
  return media === 'application/json' || media.endsWith('+json');
}

function concatBytes(chunks) {
  let total = 0;
  for (const chunk of chunks) {
    total += chunk.byteLength;
  }
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

/**
 * A10:2025 item 53. Read at most maxBytes. Declared Content-Length above
 * the cap is rejected without draining the rest of the body.
 */
export async function readTextCapped(request, maxBytes = MAX_BODY_BYTES) {
  const declared = Number.parseInt(request.headers.get('content-length') ?? '', 10);
  if (Number.isFinite(declared) && declared > maxBytes) {
    throw new BodyError(
      'payload_too_large',
      `Request body exceeds ${maxBytes} bytes.`,
      413,
    );
  }
  if (request.body && typeof request.body.getReader === 'function') {
    const reader = request.body.getReader();
    const chunks = [];
    let size = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      size += value.byteLength;
      if (size > maxBytes) {
        try {
          await reader.cancel();
        } catch {
          // Already over the cap.
        }
        throw new BodyError(
          'payload_too_large',
          `Request body exceeds ${maxBytes} bytes.`,
          413,
        );
      }
      chunks.push(value);
    }
    return new TextDecoder().decode(concatBytes(chunks));
  }
  const buf = await request.arrayBuffer();
  if (buf.byteLength > maxBytes) {
    throw new BodyError(
      'payload_too_large',
      `Request body exceeds ${maxBytes} bytes.`,
      413,
    );
  }
  return new TextDecoder().decode(buf);
}

export async function readJsonRequest(request, maxBytes = MAX_BODY_BYTES) {
  if (!isJsonContentType(request.headers.get('content-type'))) {
    throw new BodyError(
      'unsupported_media_type',
      'Send Content-Type: application/json.',
      415,
    );
  }
  const raw = await readTextCapped(request, maxBytes);
  if (!raw.trim()) {
    throw new BodyError('malformed_json', 'Request body is empty.', 400);
  }
  return { raw, payload: parseJsonStrict(raw) };
}

export function parseJsonStrict(text, maxDepth = MAX_JSON_DEPTH) {
  const source = String(text);
  const depth = jsonNestingDepth(source);
  if (depth > maxDepth) {
    throw new BodyError(
      'malformed_json',
      'JSON nesting exceeds the depth limit.',
      400,
    );
  }
  assertNoDuplicateKeys(source);
  try {
    return JSON.parse(source);
  } catch {
    throw new BodyError(
      'malformed_json',
      'Request body is not valid JSON.',
      400,
    );
  }
}

export function jsonNestingDepth(text) {
  let depth = 0;
  let max = 0;
  let inString = false;
  let escape = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (inString) {
      if (escape) {
        escape = false;
        continue;
      }
      if (ch === '\\') {
        escape = true;
        continue;
      }
      if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === '{' || ch === '[') {
      depth += 1;
      if (depth > max) {
        max = depth;
      }
    } else if (ch === '}' || ch === ']') {
      depth -= 1;
    }
  }
  return max;
}

function assertNoDuplicateKeys(text) {
  const parser = { text, i: 0 };
  skipWs(parser);
  parseValue(parser);
  skipWs(parser);
  if (parser.i !== text.length) {
    throw new BodyError(
      'malformed_json',
      'Request body is not valid JSON.',
      400,
    );
  }
}

function skipWs(p) {
  while (p.i < p.text.length) {
    const ch = p.text[p.i];
    if (ch !== ' ' && ch !== '\n' && ch !== '\r' && ch !== '\t') {
      break;
    }
    p.i += 1;
  }
}

function parseValue(p) {
  skipWs(p);
  const ch = p.text[p.i];
  if (ch === '{') {
    parseObject(p);
    return;
  }
  if (ch === '[') {
    parseArray(p);
    return;
  }
  if (ch === '"') {
    parseString(p);
    return;
  }
  if (ch === 't' || ch === 'f' || ch === 'n') {
    parseLiteral(p);
    return;
  }
  if (ch === '-' || (ch >= '0' && ch <= '9')) {
    parseNumber(p);
    return;
  }
  throw new BodyError(
    'malformed_json',
    'Request body is not valid JSON.',
    400,
  );
}

function parseObject(p) {
  p.i += 1;
  const keys = new Set();
  skipWs(p);
  if (p.text[p.i] === '}') {
    p.i += 1;
    return;
  }
  for (;;) {
    skipWs(p);
    if (p.text[p.i] !== '"') {
      throw new BodyError(
        'malformed_json',
        'Request body is not valid JSON.',
        400,
      );
    }
    const key = parseString(p);
    if (keys.has(key)) {
      throw new BodyError(
        'malformed_json',
        'JSON objects must not contain duplicate keys.',
        400,
      );
    }
    keys.add(key);
    skipWs(p);
    if (p.text[p.i] !== ':') {
      throw new BodyError(
        'malformed_json',
        'Request body is not valid JSON.',
        400,
      );
    }
    p.i += 1;
    parseValue(p);
    skipWs(p);
    const next = p.text[p.i];
    if (next === '}') {
      p.i += 1;
      return;
    }
    if (next !== ',') {
      throw new BodyError(
        'malformed_json',
        'Request body is not valid JSON.',
        400,
      );
    }
    p.i += 1;
  }
}

function parseArray(p) {
  p.i += 1;
  skipWs(p);
  if (p.text[p.i] === ']') {
    p.i += 1;
    return;
  }
  for (;;) {
    parseValue(p);
    skipWs(p);
    const next = p.text[p.i];
    if (next === ']') {
      p.i += 1;
      return;
    }
    if (next !== ',') {
      throw new BodyError(
        'malformed_json',
        'Request body is not valid JSON.',
        400,
      );
    }
    p.i += 1;
  }
}

function parseString(p) {
  p.i += 1;
  let out = '';
  while (p.i < p.text.length) {
    const ch = p.text[p.i];
    if (ch === '"') {
      p.i += 1;
      return out;
    }
    if (ch === '\\') {
      p.i += 1;
      const esc = p.text[p.i];
      p.i += 1;
      if (esc === 'u') {
        const hex = p.text.slice(p.i, p.i + 4);
        out += String.fromCharCode(Number.parseInt(hex, 16));
        p.i += 4;
        continue;
      }
      const map = { b: '\b', f: '\f', n: '\n', r: '\r', t: '\t' };
      out += map[esc] ?? esc;
      continue;
    }
    out += ch;
    p.i += 1;
  }
  throw new BodyError(
    'malformed_json',
    'Request body is not valid JSON.',
    400,
  );
}

function parseLiteral(p) {
  if (p.text.startsWith('true', p.i)) {
    p.i += 4;
    return;
  }
  if (p.text.startsWith('false', p.i)) {
    p.i += 5;
    return;
  }
  if (p.text.startsWith('null', p.i)) {
    p.i += 4;
    return;
  }
  throw new BodyError(
    'malformed_json',
    'Request body is not valid JSON.',
    400,
  );
}

function parseNumber(p) {
  const start = p.i;
  if (p.text[p.i] === '-') {
    p.i += 1;
  }
  while (p.i < p.text.length && p.text[p.i] >= '0' && p.text[p.i] <= '9') {
    p.i += 1;
  }
  if (p.text[p.i] === '.') {
    p.i += 1;
    while (p.i < p.text.length && p.text[p.i] >= '0' && p.text[p.i] <= '9') {
      p.i += 1;
    }
  }
  if (p.text[p.i] === 'e' || p.text[p.i] === 'E') {
    p.i += 1;
    if (p.text[p.i] === '+' || p.text[p.i] === '-') {
      p.i += 1;
    }
    while (p.i < p.text.length && p.text[p.i] >= '0' && p.text[p.i] <= '9') {
      p.i += 1;
    }
  }
  if (p.i === start || (p.text[start] === '-' && p.i === start + 1)) {
    throw new BodyError(
      'malformed_json',
      'Request body is not valid JSON.',
      400,
    );
  }
}
