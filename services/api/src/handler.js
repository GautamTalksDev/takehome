import changelog from './assets/changelog.json' with { type: 'json' };
import conformance from './assets/conformance.json' with { type: 'json' };
import { errorResponse, fromEngineJson, json, classifyEngineError } from './errors.js';
import { MailTransportError, sendMail } from './mail.js';
import { projectYear } from './year.js';
import { DOCS } from './schema.js';
import { latestRuleSetVersion, liveIdentity } from './identity.js';
import { buildOpenApi } from './openapi.js';
import { checkRateLimit, rateBucket } from './rate-limit.js';
import {
  AUTH_DUMMY_SECRET,
  DIGEST_PLACEHOLDER,
  equalDigest,
  hashKey,
  keyKind,
  mintKey,
  mintVerifyToken,
  mintWebhookSecret,
  parseBearer,
  newWebhookId,
  newDeliveryId,
} from './keys.js';
import { calculationCount, isBatch, batchItems, MAX_BATCH, usageHeaders } from './meter.js';
import { livePlan, periodReset, periodStart, PLANS } from './plans.js';
import { getStore } from './store.js';
import { deliverWithRetry, webhookPayload } from './webhooks.js';
import { UnsafeWebhookUrlError, assertSafeWebhookUrl } from './ssrf.js';
import { matchObjectIdRoute, objectIdMethodsFor } from './object-routes.js';
import { isPublicCorsPath, publicCorsHeaders } from './cors.js';
import {
  logException,
  observeResponse,
  recordSecurityEvent,
} from './log.js';
import {
  BodyError,
  parseJsonStrict,
  readJsonRequest,
  readTextCapped,
} from './json-body.js';

const PROBE =
  '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}';

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function pathname(url) {
  const path = new URL(url).pathname;
  if (path.length > 1 && path.endsWith('/')) {
    return path.slice(0, -1);
  }
  return path;
}

function utcDate(env) {
  if (env.CLOCK_DATE) {
    return env.CLOCK_DATE;
  }
  const now = new Date();
  const year = String(now.getUTCFullYear());
  const month = String(now.getUTCMonth() + 1).padStart(2, '0');
  const day = String(now.getUTCDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function nowMs(env) {
  return Date.parse(`${utcDate(env)}T00:00:00Z`);
}

function expiresAt(env, hours) {
  return new Date(nowMs(env) + hours * 3600 * 1000)
    .toISOString()
    .replace(/\.\d{3}Z$/, 'Z');
}

function siteUrl(env) {
  return env.PUBLIC_SITE ?? 'https://takehome.gautamkhosla.com';
}

/**
 * Dev-only signup echo. Default deny: only the exact string `"1"` opens the
 * path. Absent, empty, `"0"`, `"false"`, `"true"`, and any other value stay off.
 */
export function echoVerifyUrlEnabled(env) {
  return env?.ECHO_VERIFY_URL === '1';
}

function meterContext(env, kind, account, used) {
  const reset = periodReset(utcDate(env));
  if (kind === 'test') {
    return usageHeaders({ kind: 'test', reset });
  }
  const plan = livePlan(account.plan) ?? livePlan('developer');
  return usageHeaders({
    kind: 'live',
    limit: plan.calculations,
    used,
    reset,
  });
}

async function bodyErrorResponse(engine, err, fallbackMessage) {
  if (err instanceof BodyError) {
    return errorResponse(
      engine,
      err.code,
      err.message,
      err.status,
      DOCS[err.code] ?? DOCS.malformed_json,
    );
  }
  return errorResponse(
    engine,
    'malformed_json',
    fallbackMessage,
    400,
    DOCS.malformed_json,
  );
}

async function jsonBody(request, engine) {
  try {
    return await readJsonRequest(request);
  } catch (err) {
    return { response: await bodyErrorResponse(engine, err, 'Request body is not valid JSON.') };
  }
}

/**
 * A10:2025 item 51 / ADR-005. Metering is a control. A store failure
 * does not become a free calculation.
 */
async function storeUnavailable(engine, err) {
  try {
    logException(err, 'store_error');
  } catch {
    // Client still gets the typed 503.
  }
  return errorResponse(
    engine,
    'store',
    'The usage store is unavailable.',
    503,
    DOCS.store,
  );
}

async function readUsage(store, accountId, period) {
  try {
    return { used: await Promise.resolve(store.getUsage(accountId, period)) };
  } catch (err) {
    return { err };
  }
}

async function writeUsage(store, accountId, period, n) {
  try {
    return { billed: await Promise.resolve(store.addUsage(accountId, period, n)) };
  } catch (err) {
    return { err };
  }
}

function applyNoindex(env, response) {
  if (env?.NOINDEX !== '1') {
    return response;
  }
  const headers = new Headers(response.headers);
  headers.set('x-robots-tag', 'noindex, nofollow');
  return new Response(response.body, {
    status: response.status,
    headers,
  });
}

export async function handle(request, env, engine) {
  try {
    const response = applyNoindex(
      env,
      applyPublicCors(request, await dispatch(request, env, engine)),
    );
    try {
      await observeResponse(request, env, response);
    } catch {
      // Logging must never change the response.
    }
    return response;
  } catch (err) {
    try {
      logException(err, 'engine_panic');
    } catch {
      // Logging must never change the response.
    }
    try {
      await recordSecurityEvent(
        env,
        { type: 'http_5xx', status: 500, reason: 'unhandled' },
        request,
      );
    } catch {
      // Logging must never change the response.
    }
    try {
      return applyNoindex(
        env,
        applyPublicCors(
          request,
          await errorResponse(
            engine,
            'engine',
            'The request could not be completed.',
            500,
            DOCS.engine,
          ),
        ),
      );
    } catch {
      return applyNoindex(
        env,
        new Response(
          JSON.stringify({
            error: {
              code: 'engine',
              message: 'The request could not be completed.',
              docs: DOCS.engine,
            },
          }),
          {
            status: 500,
            headers: { 'content-type': 'application/json; charset=utf-8' },
          },
        ),
      );
    }
  }
}

function applyPublicCors(request, response) {
  const path = pathname(request.url);
  if (!isPublicCorsPath(path)) {
    return response;
  }
  const headers = new Headers(response.headers);
  for (const [name, value] of Object.entries(publicCorsHeaders())) {
    headers.set(name, value);
  }
  return new Response(response.body, {
    status: response.status,
    headers,
  });
}

async function dispatch(request, env, engine) {
  const path = pathname(request.url);
  if (request.method === 'OPTIONS') {
    if (isPublicCorsPath(path)) {
      return new Response(null, {
        status: 204,
        headers: publicCorsHeaders(),
      });
    }
    return new Response(null, { status: 204 });
  }

  const bucket = rateBucket(path, request.method);
  if (bucket) {
    const limited = checkRateLimit(request, env, bucket);
    if (!limited.allowed) {
      return errorResponse(
        engine,
        'rate_limited',
        `IP rate limit exceeded (${limited.limit} requests per window).`,
        429,
        DOCS.rate_limited,
      );
    }
  }

  if (request.method === 'GET' && path === '/health') {
    return health(engine);
  }
  if (request.method === 'GET' && path === '/openapi.json') {
    return openapi(engine);
  }
  if (request.method === 'GET' && path === '/v1/jurisdictions') {
    return jurisdictions(engine);
  }
  if (request.method === 'GET' && path === '/v1/changes') {
    return changes(engine);
  }
  if (request.method === 'GET' && path === '/v1/changes.rss') {
    return changesRss();
  }
  if (request.method === 'GET' && path === '/v1/conformance') {
    return conformanceRecord(engine);
  }
  if (request.method === 'GET' && path === '/v1/rules') {
    return rulesList(engine);
  }
  if (request.method === 'GET' && path === '/v1/rules/diff') {
    return rulesDiff(request, engine);
  }
  const ruleMatch = path.match(/^\/v1\/rules\/([^/]+)$/);
  if (request.method === 'GET' && ruleMatch) {
    return ruleVersion(engine, decodeURIComponent(ruleMatch[1]));
  }
  if (path === '/v1/deductions') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/deductions. A pay run is POST /v1/deductions/batch.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return deductions(request, env, engine);
  }
  if (path === '/v1/deductions/batch') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/deductions/batch with { "requests": [ ... ] }, up to 1000 items.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return deductionsBatch(request, env, engine);
  }
  if (path === '/v1/deductions/year') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/deductions/year with a first pay date, province, pay_period, and gross_pay.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return deductionsYear(request, env, engine);
  }
  if (path === '/v1/signup') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/signup with { email }.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return signup(request, env, engine);
  }
  if (path === '/v1/signup/verify') {
    if (request.method !== 'GET') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'GET /v1/signup/verify?token=…',
        405,
        DOCS.method_not_allowed,
      );
    }
    return verifySignup(request, env, engine);
  }
  if (path === '/v1/billing/checkout') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/billing/checkout with a live key.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return checkout(request, env, engine);
  }
  if (path === '/v1/billing/webhook') {
    // ADR-006: billing deferred. Do not accept signed Stripe events.
    return errorResponse(
      engine,
      'not_found',
      `No endpoint at ${path}.`,
      404,
      DOCS.not_found,
    );
  }
  if (path === '/v1/keys') {
    if (request.method !== 'GET') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'GET /v1/keys.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return listAccountKeys(request, env, engine);
  }
  if (path === '/v1/keys/rotate') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/keys/rotate with { "kind": "test" } or { "kind": "live" }.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return rotateAccountKey(request, env, engine);
  }
  if (path === '/v1/keys/revoke') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/keys/revoke with { "kind": "test" } or { "kind": "live" }.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return revokeAccountKey(request, env, engine);
  }
  if (path === '/v1/webhooks') {
    if (request.method === 'GET') {
      return listWebhookEndpoints(request, env, engine);
    }
    if (request.method === 'POST') {
      return createWebhookEndpoint(request, env, engine);
    }
    return errorResponse(
      engine,
      'method_not_allowed',
      'GET or POST /v1/webhooks.',
      405,
      DOCS.method_not_allowed,
    );
  }
  if (path === '/v1/webhooks/dispatch') {
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/webhooks/dispatch with { from, to }.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return dispatchWebhooks(request, env, engine);
  }
  if (path === '/v1/webhooks/deliveries') {
    if (request.method !== 'GET') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'GET /v1/webhooks/deliveries.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return listWebhookDeliveries(request, env, engine);
  }
  const OBJECT_ID_HANDLERS = {
    getWebhook: getWebhookEndpoint,
    deleteWebhook: deleteWebhookEndpoint,
    replayDelivery: replayWebhookDelivery,
  };
  const objectHit = matchObjectIdRoute(request.method, path);
  if (objectHit) {
    const fn = OBJECT_ID_HANDLERS[objectHit.route.name];
    if (!fn) {
      return errorResponse(
        engine,
        'not_found',
        `No endpoint at ${path}.`,
        404,
        DOCS.not_found,
      );
    }
    return fn(request, env, engine, objectHit.id);
  }
  const objectMethods = objectIdMethodsFor(path);
  if (objectMethods.length > 0) {
    return errorResponse(
      engine,
      'method_not_allowed',
      `${objectMethods.join(' or ')} ${path}.`,
      405,
      DOCS.method_not_allowed,
    );
  }

  return errorResponse(
    engine,
    'not_found',
    `No endpoint at ${path}.`,
    404,
    DOCS.not_found,
  );
}

async function unauthorized(engine) {
  return errorResponse(
    engine,
    'unauthorized',
    'Authorization: Bearer np_test_… or np_live_… is required.',
    401,
    DOCS.unauthorized,
    { 'www-authenticate': 'Bearer' },
  );
}

async function authenticate(request, env, engine) {
  const secret = parseBearer(request) ?? '';
  const kind = keyKind(secret);
  const presented = hashKey(kind ? secret : AUTH_DUMMY_SECRET);
  const store = getStore(env);
  if (!store) {
    return {
      response: await errorResponse(
        engine,
        'store',
        'API key store is not configured.',
        503,
        DOCS.engine,
      ),
    };
  }
  const row = await Promise.resolve(store.getKeyByHash(presented));
  const stored = row ? row.hash : DIGEST_PLACEHOLDER;
  const digestOk = equalDigest(presented, stored);
  const revoked = Boolean(row?.revoked);
  if (!kind || !digestOk || !row || row.kind !== kind || revoked) {
    return { response: await unauthorized(engine) };
  }
  const account = await Promise.resolve(store.getAccount(row.account_id));
  if (!account) {
    return { response: await unauthorized(engine) };
  }
  return { store, account, kind };
}

function requestedKeyKind(payload) {
  return payload?.kind === 'test' || payload?.kind === 'live'
    ? payload.kind
    : null;
}

async function listAccountKeys(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const rows = await Promise.resolve(auth.store.listKeys(auth.account.id));
  const identity = await liveIdentity(engine, null);
  return json(200, {
    ...identity,
    keys: rows
      .filter((row) => !row.revoked)
      .map((row) => ({ kind: row.kind, prefix: row.prefix })),
  });
}

async function rotateAccountKey(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch (err) {
    return bodyErrorResponse(
      engine,
      err,
      'Rotate expects JSON { "kind": "test" } or { "kind": "live" }.',
    );
  }
  const kind = requestedKeyKind(payload);
  if (!kind) {
    return errorResponse(
      engine,
      'invalid_request',
      'Rotate expects JSON { "kind": "test" } or { "kind": "live" }.',
      400,
      DOCS.invalid_request,
    );
  }
  const next = mintKey(kind);
  await Promise.resolve(auth.store.revokeKind(auth.account.id, kind));
  await Promise.resolve(
    auth.store.insertKey({
      account_id: auth.account.id,
      kind,
      prefix: `np_${kind}_`,
      hash: hashKey(next),
    }),
  );
  return json(200, {
    kind,
    key: next,
    message: 'Shown once. The previous key of this kind no longer works.',
  });
}

async function revokeAccountKey(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch (err) {
    return bodyErrorResponse(
      engine,
      err,
      'Revoke expects JSON { "kind": "test" } or { "kind": "live" }.',
    );
  }
  const kind = requestedKeyKind(payload) ?? auth.kind;
  await Promise.resolve(auth.store.revokeKind(auth.account.id, kind));
  return json(200, { kind, revoked: true });
}

async function deductions(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const { store, account, kind } = auth;
  const period = periodStart(utcDate(env));
  const usage = await readUsage(store, account.id, period);
  if (usage.err) {
    return storeUnavailable(engine, usage.err);
  }
  const used = usage.used;
  const plan = livePlan(account.plan) ?? livePlan('developer');

  const parsedBody = await jsonBody(request, engine);
  if (parsedBody.response) {
    return parsedBody.response;
  }
  const { raw } = parsedBody;
  let { payload } = parsedBody;
  const n = payload == null ? 1 : calculationCount(payload);

  if (kind === 'live') {
    if (used + n > plan.calculations) {
      return planLimitResponse(engine, env, kind, account, used, n, plan);
    }
  }

  const headers = meterContext(
    env,
    kind,
    account,
    kind === 'live' ? used : 0,
  );

  if (payload != null && isBatch(payload)) {
    return errorResponse(
      engine,
      'invalid_request',
      `POST /v1/deductions is one calculation. Send a batch of ${n} to POST /v1/deductions/batch (max ${MAX_BATCH}).`,
      400,
      DOCS.invalid_request,
      headers,
    );
  }

  let calculateInput = raw;
  if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
    if (payload.as_of == null) {
      payload = { ...payload, as_of: utcDate(env) };
    }
    calculateInput = JSON.stringify(payload);
  }
  const body = await Promise.resolve(engine.calculate(calculateInput));
  const parsed = JSON.parse(body);
  if (parsed.error) {
    return fromEngineJson(body, engine, headers);
  }
  let billed = used;
  if (kind === 'live') {
    const wrote = await writeUsage(store, account.id, period, n);
    if (wrote.err) {
      return storeUnavailable(engine, wrote.err);
    }
    billed = wrote.billed;
  }
  return fromEngineJson(body, engine, meterContext(env, kind, account, billed));
}

async function planLimitResponse(engine, env, kind, account, used, n, plan) {
  const headers = meterContext(env, kind, account, used);
  return errorResponse(
    engine,
    'plan_limit',
    `Live plan ${plan.name} includes ${plan.calculations} calculations this month. This request is ${n} calculation${n === 1 ? '' : 's'}. Upgrade at ${PLANS.upgrade_url} — Takehome does not surprise-bill.`,
    402,
    DOCS.plan_limit,
    headers,
  );
}

function itemError(code, message, docs) {
  return { error: { code, message, docs } };
}

async function calculateOne(item, env, engine) {
  if (item == null || typeof item !== 'object' || Array.isArray(item)) {
    return itemError(
      'invalid_request',
      'Each batch item must be a T4127 deduction request object.',
      DOCS.invalid_request,
    );
  }
  const payload = item.as_of == null ? { ...item, as_of: utcDate(env) } : item;
  const body = await Promise.resolve(engine.calculate(JSON.stringify(payload)));
  const parsed = JSON.parse(body);
  if (parsed.error) {
    const mapped = classifyEngineError(parsed.error.code, parsed.error.message);
    return itemError(mapped.code, parsed.error.message, mapped.docs);
  }
  return { ok: true, response: parsed };
}

async function deductionsBatch(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const { store, account, kind } = auth;
  const period = periodStart(utcDate(env));
  const usage = await readUsage(store, account.id, period);
  if (usage.err) {
    return storeUnavailable(engine, usage.err);
  }
  const used = usage.used;
  const plan = livePlan(account.plan) ?? livePlan('developer');

  const parsedBody = await jsonBody(request, engine);
  if (parsedBody.response) {
    return parsedBody.response;
  }
  const payload = parsedBody.payload;
  const items = batchItems(payload);
  if (!items) {
    return errorResponse(
      engine,
      'invalid_request',
      'POST /v1/deductions/batch expects { "requests": [ ... ] } with 1 to 1000 calculations.',
      400,
      DOCS.invalid_request,
    );
  }
  const n = items.length;
  if (n < 1 || n > MAX_BATCH) {
    return errorResponse(
      engine,
      n > MAX_BATCH ? 'batch_too_large' : 'invalid_request',
      n > MAX_BATCH
        ? `This batch has ${n} calculations. The limit is ${MAX_BATCH} per call.`
        : `POST /v1/deductions/batch expects { "requests": [ ... ] } with 1 to ${MAX_BATCH} calculations.`,
      400,
      n > MAX_BATCH ? DOCS.batch_too_large : DOCS.invalid_request,
    );
  }

  if (kind === 'live' && used + n > plan.calculations) {
    return planLimitResponse(engine, env, kind, account, used, n, plan);
  }

  const results = [];
  for (const item of items) {
    results.push(await calculateOne(item, env, engine));
  }

  let billed = used;
  if (kind === 'live') {
    const wrote = await writeUsage(store, account.id, period, n);
    if (wrote.err) {
      return storeUnavailable(engine, wrote.err);
    }
    billed = wrote.billed;
  }
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, results }, meterContext(env, kind, account, billed));
}

async function deductionsYear(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const { store, account, kind } = auth;
  const period = periodStart(utcDate(env));
  const usage = await readUsage(store, account.id, period);
  if (usage.err) {
    return storeUnavailable(engine, usage.err);
  }
  const used = usage.used;
  const plan = livePlan(account.plan) ?? livePlan('developer');

  const parsedBody = await jsonBody(request, engine);
  if (parsedBody.response) {
    return parsedBody.response;
  }
  let payload = parsedBody.payload;
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return errorResponse(
      engine,
      'invalid_request',
      'Year projection expects a single T4127 request: first pay date, province, pay_period, gross_pay.',
      400,
      DOCS.invalid_request,
    );
  }
  if (payload.as_of == null) {
    payload = { ...payload, as_of: utcDate(env) };
  }
  const n = Number.parseInt(String(payload.pay_period), 10);
  if (!Number.isInteger(n) || n < 1) {
    return errorResponse(
      engine,
      'invalid_request',
      'pay_period is required so the year knows how many cheques to run.',
      400,
      DOCS.invalid_request,
    );
  }
  if (kind === 'live' && used + n > plan.calculations) {
    return planLimitResponse(engine, env, kind, account, used, n, plan);
  }

  const projected = await projectYear(payload, engine);
  if (projected.error) {
    const mapped = classifyEngineError(
      projected.error.code,
      projected.error.message,
    );
    return errorResponse(
      engine,
      mapped.code,
      projected.error.message,
      mapped.status,
      mapped.docs,
    );
  }

  let billed = used;
  if (kind === 'live') {
    const wrote = await writeUsage(store, account.id, period, n);
    if (wrote.err) {
      return storeUnavailable(engine, wrote.err);
    }
    billed = wrote.billed;
  }
  const identity = await liveIdentity(engine, null);
  return json(
    200,
    { ...identity, ...projected },
    meterContext(env, kind, account, billed),
  );
}

async function signup(request, env, engine) {
  const store = getStore(env);
  if (!store) {
    return errorResponse(
      engine,
      'store',
      'API key store is not configured.',
      503,
      DOCS.engine,
    );
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch (err) {
    return bodyErrorResponse(
      engine,
      err,
      'Signup expects JSON { "email": "…" }.',
    );
  }
  const rawEmail = String(payload?.email ?? '');
  if (/[\r\n\0]/.test(rawEmail)) {
    return errorResponse(
      engine,
      'invalid_request',
      'Provide an email address. Keys are issued after you verify it.',
      400,
      DOCS.invalid_request,
    );
  }
  const email = rawEmail.trim().toLowerCase();
  if (!EMAIL.test(email)) {
    return errorResponse(
      engine,
      'invalid_request',
      'Provide an email address. Keys are issued after you verify it.',
      400,
      DOCS.invalid_request,
    );
  }
  let account = await Promise.resolve(store.getAccountByEmail(email));
  if (!account) {
    account = await Promise.resolve(
      store.createAccount({
        email,
        email_verified: false,
        plan: 'developer',
      }),
    );
  }
  const message =
    'Check your email. After you verify, you receive a test key and a live key and can make the first call.';
  if (account.email_verified) {
    return json(200, { message });
  }
  const token = mintVerifyToken();
  await Promise.resolve(
    store.insertEmailToken({
      hash: hashKey(token),
      account_id: account.id,
      expires_at: expiresAt(env, 24),
    }),
  );
  const verifyUrl = `${siteUrl(env)}/signup/verify/?token=${token}`;
  try {
    await sendMail(env, {
      to: email,
      subject: 'Verify your Takehome email',
      text: `Verify and receive API keys: ${verifyUrl}`,
      token,
      verifyUrl,
    });
  } catch (err) {
    if (err instanceof MailTransportError) {
      await Promise.resolve(store.deleteEmailToken(hashKey(token))).catch(() => {});
      return errorResponse(
        engine,
        'mail',
        'Verification email could not be sent. Try again later.',
        503,
        DOCS.mail,
      );
    }
    throw err;
  }
  const body = { message };
  if (echoVerifyUrlEnabled(env)) {
    body.verify_url = verifyUrl;
  }
  return json(200, body);
}

async function verifySignup(request, env, engine) {
  const store = getStore(env);
  if (!store) {
    return errorResponse(
      engine,
      'store',
      'API key store is not configured.',
      503,
      DOCS.engine,
    );
  }
  const token = new URL(request.url).searchParams.get('token') ?? '';
  if (!token) {
    return errorResponse(
      engine,
      'invalid_request',
      'Missing verification token.',
      400,
      DOCS.invalid_request,
    );
  }
  const presented = hashKey(token);
  const row = await Promise.resolve(store.getEmailToken(presented));
  const stored = row ? row.hash : DIGEST_PLACEHOLDER;
  if (!equalDigest(presented, stored) || !row) {
    return errorResponse(
      engine,
      'invalid_request',
      'That verification token is not valid.',
      400,
      DOCS.invalid_request,
    );
  }
  await Promise.resolve(store.deleteEmailToken(row.hash));
  if (Date.parse(row.expires_at) < nowMs(env)) {
    return errorResponse(
      engine,
      'invalid_request',
      'That verification token has expired. Sign up again.',
      400,
      DOCS.invalid_request,
    );
  }
  const existing = await Promise.resolve(store.listKeys(row.account_id));
  if (existing.some((key) => key.kind === 'live' || key.kind === 'test')) {
    await Promise.resolve(store.deleteEmailTokensForAccount(row.account_id));
    return errorResponse(
      engine,
      'invalid_request',
      'That verification token is not valid.',
      400,
      DOCS.invalid_request,
    );
  }
  await Promise.resolve(store.setVerified(row.account_id));
  const testKey = mintKey('test');
  const liveKey = mintKey('live');
  await Promise.resolve(
    store.insertKey({
      account_id: row.account_id,
      kind: 'test',
      prefix: 'np_test_',
      hash: hashKey(testKey),
    }),
  );
  await Promise.resolve(
    store.insertKey({
      account_id: row.account_id,
      kind: 'live',
      prefix: 'np_live_',
      hash: hashKey(liveKey),
    }),
  );
  await Promise.resolve(store.deleteEmailTokensForAccount(row.account_id));
  return json(200, {
    test_key: testKey,
    live_key: liveKey,
    message:
      'Email verified. Use the test key while you integrate — same engine, unmetered. The live key counts against your plan.',
  });
}

/**
 * ADR-006: paid checkout is deferred. Authenticate first so the unauthenticated
 * surface stays 401; a live key gets a typed 501 rather than a Stripe session.
 */
async function checkout(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  if (auth.kind !== 'live') {
    return unauthorized(engine);
  }
  return errorResponse(
    engine,
    'billing_unavailable',
    'Billing is deferred. Every live tier is free during early access. See the pricing page for the monthly quota and the published paid tiers that will return later.',
    501,
    DOCS.billing_unavailable,
  );
}

async function readBody(request) {
  const type = request.headers.get('content-type') ?? '';
  const raw = await readTextCapped(request);
  if (type.includes('application/x-www-form-urlencoded')) {
    return Object.fromEntries(new URLSearchParams(raw));
  }
  if (!raw) {
    return {};
  }
  return parseJsonStrict(raw);
}

async function health(engine) {
  const raw = await Promise.resolve(engine.calculate(PROBE));
  const parsed = JSON.parse(raw);
  if (parsed.error) {
    return fromEngineJson(raw, engine);
  }
  if (parsed.employee?.net_pay !== '800.79') {
    return errorResponse(
      engine,
      'engine',
      'health probe net_pay did not match the M1 Ontario weekly vector',
      503,
      DOCS.engine,
    );
  }
  return json(200, { status: 'ok' });
}

async function openapi(engine) {
  const latest = await latestRuleSetVersion(engine);
  const identity = await liveIdentity(engine, latest);
  return json(200, { ...identity, document: buildOpenApi() });
}

async function jurisdictions(engine) {
  const listing = JSON.parse(
    await Promise.resolve(engine.listJurisdictions()),
  );
  const latest = await latestRuleSetVersion(engine);
  const identity = await liveIdentity(engine, latest);
  return json(200, { ...identity, jurisdictions: listing.jurisdictions });
}

async function changes(engine) {
  const latest = await latestRuleSetVersion(engine);
  const identity = await liveIdentity(engine, latest);
  return json(200, { ...identity, ...changelog });
}

async function conformanceRecord(engine) {
  const latest = await latestRuleSetVersion(engine);
  const identity = await liveIdentity(engine, latest);
  return json(200, { ...identity, record: conformance });
}

async function rulesList(engine) {
  const listing = JSON.parse(
    await Promise.resolve(engine.listRuleSetVersions()),
  );
  const latest = listing.rule_set_versions.at(-1).version;
  const identity = await liveIdentity(engine, latest);
  return json(200, { ...identity, rule_set_versions: listing.rule_set_versions });
}

async function ruleVersion(engine, version) {
  const listing = JSON.parse(
    await Promise.resolve(engine.listRuleSetVersions()),
  );
  const row = listing.rule_set_versions.find((item) => item.version === version);
  if (!row) {
    return errorResponse(
      engine,
      'not_found',
      `Unknown rule set ${version}. This API does not nearest-match editions.`,
      404,
      DOCS.not_found,
    );
  }
  const identity = await liveIdentity(engine, row.version);
  return json(200, {
    ...identity,
    effective_from: row.effective_from,
    effective_to: row.effective_to ?? null,
    status: row.status ?? 'enacted',
  });
}

async function rulesDiff(request, engine) {
  const params = new URL(request.url).searchParams;
  const from = params.get('from');
  const to = params.get('to');
  if (!from || !to) {
    return errorResponse(
      engine,
      'invalid_request',
      'GET /v1/rules/diff requires from and to rule-set versions (YYYY-MM-DD).',
      400,
      DOCS.invalid_request,
    );
  }
  if (typeof engine.diffRuleSets !== 'function') {
    return errorResponse(
      engine,
      'engine',
      'This engine build cannot diff rule sets.',
      500,
      DOCS.engine,
    );
  }
  const raw = await Promise.resolve(engine.diffRuleSets(from, to));
  const parsed = JSON.parse(raw);
  if (parsed.error) {
    return errorResponse(
      engine,
      'not_found',
      parsed.error.message,
      404,
      DOCS.not_found,
    );
  }
  const identity = await liveIdentity(engine, to);
  return json(200, { ...identity, ...parsed });
}

function isoNow(env) {
  return new Date(nowMs(env)).toISOString().replace(/\.\d{3}Z$/, 'Z');
}

function unixSec(env) {
  return Math.floor(nowMs(env) / 1000);
}

function webhookTimeoutMs(env) {
  const raw = env.WEBHOOK_TIMEOUT_MS;
  if (raw == null || raw === '') {
    return undefined;
  }
  const n = Number.parseInt(String(raw), 10);
  return Number.isInteger(n) && n > 0 ? n : undefined;
}

function webhookTransport(env) {
  return {
    fetchImpl: env.WEBHOOK_FETCH ?? globalThis.fetch.bind(globalThis),
    sleep:
      env.WEBHOOK_SLEEP ??
      ((ms) =>
        new Promise((resolve) => {
          setTimeout(resolve, ms);
        })),
  };
}

function publicWebhook(row) {
  return {
    id: row.id,
    url: row.url,
    created_at: row.created_at,
  };
}

function publicDelivery(row) {
  return {
    id: row.id,
    endpoint_id: row.endpoint_id,
    event: row.event,
    status: row.status,
    attempts: row.attempts,
    last_error: row.last_error ?? null,
    replay_of: row.replay_of ?? null,
    created_at: row.created_at,
    delivered_at: row.delivered_at ?? null,
  };
}

async function createWebhookEndpoint(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch (err) {
    return bodyErrorResponse(
      engine,
      err,
      'Register a webhook with JSON { "url": "https://…" }.',
    );
  }
  const url = String(payload?.url ?? '');
  try {
    await assertSafeWebhookUrl(url, { resolve: env.WEBHOOK_RESOLVE });
  } catch (err) {
    const message =
      err instanceof UnsafeWebhookUrlError
        ? err.message
        : 'Webhook URL must be https.';
    return errorResponse(
      engine,
      'invalid_request',
      message,
      400,
      DOCS.invalid_request,
    );
  }
  const row = {
    id: newWebhookId(),
    account_id: auth.account.id,
    url,
    secret: mintWebhookSecret(),
    created_at: isoNow(env),
  };
  await Promise.resolve(auth.store.insertWebhook(row));
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, ...publicWebhook(row), secret: row.secret });
}

async function listWebhookEndpoints(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const rows = await Promise.resolve(auth.store.listWebhooks(auth.account.id));
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, webhooks: rows.map(publicWebhook) });
}

async function getWebhookEndpoint(request, env, engine, id) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const row = await Promise.resolve(auth.store.getWebhook(id, auth.account.id));
  if (!row) {
    return errorResponse(
      engine,
      'not_found',
      `No webhook ${id}.`,
      404,
      DOCS.not_found,
    );
  }
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, ...publicWebhook(row) });
}

async function deleteWebhookEndpoint(request, env, engine, id) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const deleted = await Promise.resolve(
    auth.store.deleteWebhook(id, auth.account.id),
  );
  if (!deleted) {
    return errorResponse(
      engine,
      'not_found',
      `No webhook ${id}.`,
      404,
      DOCS.not_found,
    );
  }
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, deleted: true, id });
}

async function listWebhookDeliveries(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const rows = await Promise.resolve(auth.store.listDeliveries(auth.account.id));
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, deliveries: rows.map(publicDelivery) });
}

async function dispatchWebhooks(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch (err) {
    return bodyErrorResponse(
      engine,
      err,
      'Dispatch expects JSON { "from": "YYYY-MM-DD", "to": "YYYY-MM-DD" }.',
    );
  }
  const from = String(payload?.from ?? '');
  const to = String(payload?.to ?? '');
  if (!from || !to) {
    return errorResponse(
      engine,
      'invalid_request',
      'POST /v1/webhooks/dispatch requires from and to rule-set versions.',
      400,
      DOCS.invalid_request,
    );
  }
  const raw = await Promise.resolve(engine.diffRuleSets(from, to));
  const diff = JSON.parse(raw);
  if (diff.error) {
    return errorResponse(
      engine,
      'not_found',
      diff.error.message,
      404,
      DOCS.not_found,
    );
  }
  const body = webhookPayload({
    from,
    to,
    changed: diff.changed,
    occurredAt: isoNow(env),
  });
  const endpoints = await Promise.resolve(
    auth.store.listWebhooks(auth.account.id),
  );
  const deliveries = [];
  for (const endpoint of endpoints) {
    deliveries.push(await sendWebhook(env, auth.store, endpoint, body, null));
  }
  const identity = await liveIdentity(engine, to);
  return json(200, {
    ...identity,
    from,
    to,
    changed: diff.changed,
    deliveries: deliveries.map(publicDelivery),
  });
}

async function replayWebhookDelivery(request, env, engine, id) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const original = await Promise.resolve(
    auth.store.getDelivery(id, auth.account.id),
  );
  if (!original) {
    return errorResponse(
      engine,
      'not_found',
      `No delivery ${id}.`,
      404,
      DOCS.not_found,
    );
  }
  const endpoint = await Promise.resolve(
    auth.store.getWebhook(original.endpoint_id, auth.account.id),
  );
  if (!endpoint) {
    return errorResponse(
      engine,
      'not_found',
      `No delivery ${id}.`,
      404,
      DOCS.not_found,
    );
  }
  const replayed = await sendWebhook(
    env,
    auth.store,
    endpoint,
    original.payload,
    original.id,
  );
  const identity = await liveIdentity(engine, null);
  return json(200, { ...identity, ...publicDelivery(replayed) });
}

async function sendWebhook(env, store, endpoint, body, replayOf) {
  const createdAt = isoNow(env);
  const row = {
    id: newDeliveryId(),
    endpoint_id: endpoint.id,
    account_id: endpoint.account_id,
    event: 'rule_set.changed',
    payload: body,
    status: 'pending',
    attempts: 0,
    last_error: null,
    last_http_status: null,
    replay_of: replayOf,
    created_at: createdAt,
    delivered_at: null,
  };
  await Promise.resolve(store.insertDelivery(row));
  try {
    await assertSafeWebhookUrl(endpoint.url, { resolve: env.WEBHOOK_RESOLVE });
  } catch (err) {
    const patch = {
      status: 'failed',
      attempts: 0,
      last_error:
        err instanceof UnsafeWebhookUrlError
          ? err.message
          : 'Webhook URL failed safety checks.',
      last_http_status: null,
      delivered_at: null,
    };
    try {
      await recordSecurityEvent(env, {
        type: 'webhook_delivery_failed',
        endpoint_id: endpoint.id,
        last_error: patch.last_error,
        path: '/v1/webhooks/dispatch',
      });
    } catch {
      // Delivery status is already recorded; alerting must not throw out.
    }
    return (
      (await Promise.resolve(store.updateDelivery(row.id, patch))) ?? {
        ...row,
        ...patch,
      }
    );
  }
  const { fetchImpl, sleep } = webhookTransport(env);
  const result = await deliverWithRetry({
    url: endpoint.url,
    body,
    secret: endpoint.secret,
    timestampSec: unixSec(env),
    fetchImpl,
    sleep,
    timeoutMs: webhookTimeoutMs(env),
  });
  const patch = {
    status: result.ok ? 'delivered' : 'failed',
    attempts: result.attempts,
    last_error: result.error,
    last_http_status: result.status,
    delivered_at: result.ok ? isoNow(env) : null,
  };
  if (patch.status === 'failed') {
    try {
      await recordSecurityEvent(env, {
        type: 'webhook_delivery_failed',
        endpoint_id: endpoint.id,
        last_http_status: patch.last_http_status,
        last_error: patch.last_error,
        path: '/v1/webhooks/dispatch',
      });
    } catch {
      // Delivery status is already recorded; alerting must not throw out.
    }
  }
  return (
    (await Promise.resolve(store.updateDelivery(row.id, patch))) ?? {
      ...row,
      ...patch,
    }
  );
}

function changesRss() {
  const items = changelog.items
    .map(
      (item) => `    <item>
      <title>${escapeXml(item.title)}</title>
      <link>${escapeXml(item.link)}</link>
      <guid isPermaLink="false">${escapeXml(item.id)}</guid>
      <pubDate>${rfc822(item.date)}</pubDate>
      <description>${escapeXml(item.summary)}</description>
    </item>`,
    )
    .join('\n');
  const body = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>${escapeXml(changelog.feed_title)}</title>
    <link>${escapeXml(changelog.feed_link)}</link>
    <description>${escapeXml(changelog.feed_description)}</description>
${items}
  </channel>
</rss>
`;
  return new Response(body, {
    headers: {
      'content-type': 'application/rss+xml; charset=utf-8',
    },
  });
}

function escapeXml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function rfc822(isoDate) {
  const [year, month, day] = isoDate.split('-');
  const months = [
    'Jan',
    'Feb',
    'Mar',
    'Apr',
    'May',
    'Jun',
    'Jul',
    'Aug',
    'Sep',
    'Oct',
    'Nov',
    'Dec',
  ];
  const monthName = months[Number.parseInt(month, 10) - 1];
  return `${day} ${monthName} ${year} 00:00:00 +0000`;
}

