import changelog from './assets/changelog.json' with { type: 'json' };
import conformance from './assets/conformance.json' with { type: 'json' };
import { corsHeaders, errorResponse, fromEngineJson, json } from './errors.js';
import { DOCS } from './schema.js';
import { latestRuleSetVersion, liveIdentity } from './identity.js';
import { buildOpenApi } from './openapi.js';
import { checkRateLimit } from './rate-limit.js';
import { hashKey, keyKind, mintKey, parseBearer, randomHex } from './keys.js';
import { calculationCount, isBatch, usageHeaders } from './meter.js';
import { livePlan, periodReset, periodStart, PLANS } from './plans.js';
import { getStore } from './store.js';
import { stripeClient } from './stripe.js';

const PROBE =
  '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}';

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const PAID_PLANS = new Set(['starter', 'growth', 'business']);
const PRICE_ENV = {
  starter: 'STRIPE_PRICE_STARTER',
  growth: 'STRIPE_PRICE_GROWTH',
  business: 'STRIPE_PRICE_BUSINESS',
};

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

export async function handle(request, env, engine) {
  if (request.method === 'OPTIONS') {
    return new Response(null, { status: 204, headers: corsHeaders() });
  }

  const limited = checkRateLimit(request, env);
  if (!limited.allowed) {
    return errorResponse(
      engine,
      'rate_limited',
      `IP rate limit exceeded (${limited.limit} requests per window).`,
      429,
      DOCS.rate_limited,
    );
  }

  const path = pathname(request.url);
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
  if (request.method === 'GET' && path === '/v1/conformance') {
    return conformanceRecord(engine);
  }
  if (request.method === 'GET' && path === '/v1/rules') {
    return rulesList(engine);
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
        'POST /v1/deductions. Batch endpoints are M4.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return deductions(request, env, engine);
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
    if (request.method !== 'POST') {
      return errorResponse(
        engine,
        'method_not_allowed',
        'POST /v1/billing/webhook is the Stripe endpoint.',
        405,
        DOCS.method_not_allowed,
      );
    }
    return webhook(request, env, engine);
  }

  return errorResponse(
    engine,
    'not_found',
    `No M3 endpoint at ${path}. Batch and year endpoints are M4.`,
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
  const secret = parseBearer(request);
  if (!secret) {
    return { response: await unauthorized(engine) };
  }
  const kind = keyKind(secret);
  if (!kind) {
    return { response: await unauthorized(engine) };
  }
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
  const row = await Promise.resolve(store.getKeyByHash(hashKey(secret)));
  if (!row || row.kind !== kind) {
    return { response: await unauthorized(engine) };
  }
  const account = await Promise.resolve(store.getAccount(row.account_id));
  if (!account) {
    return { response: await unauthorized(engine) };
  }
  return { store, account, kind };
}

async function deductions(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  const { store, account, kind } = auth;
  const period = periodStart(utcDate(env));
  const used = await Promise.resolve(store.getUsage(account.id, period));
  const plan = livePlan(account.plan) ?? livePlan('developer');

  const raw = await request.text();
  let payload;
  try {
    payload = JSON.parse(raw);
  } catch {
    payload = null;
  }
  const n = payload == null ? 1 : calculationCount(payload);

  if (kind === 'live') {
    if (used + n > plan.calculations) {
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
      'batch_not_implemented',
      `A batch of ${n} calculations is M4. Metering already counts ${n}, not one HTTP request.`,
      400,
      DOCS.batch_not_implemented,
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
    billed = await Promise.resolve(store.addUsage(account.id, period, n));
  }
  return fromEngineJson(body, engine, meterContext(env, kind, account, billed));
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
  } catch {
    return errorResponse(
      engine,
      'malformed_json',
      'Signup expects JSON { "email": "…" }.',
      400,
      DOCS.malformed_json,
    );
  }
  const email = String(payload?.email ?? '')
    .trim()
    .toLowerCase();
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
  const token = randomHex(24);
  await Promise.resolve(
    store.insertEmailToken({
      hash: hashKey(token),
      account_id: account.id,
      expires_at: expiresAt(env, 24),
    }),
  );
  const verifyUrl = `${siteUrl(env)}/signup/verify/?token=${token}`;
  await sendMail(env, {
    to: email,
    subject: 'Verify your Takehome email',
    text: `Verify and receive API keys: ${verifyUrl}`,
    token,
    verifyUrl,
  });
  const body = { message };
  if (env.ECHO_VERIFY_URL === '1' || env.ECHO_VERIFY_URL === 'true') {
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
  const row = await Promise.resolve(store.consumeEmailToken(hashKey(token)));
  if (!row) {
    return errorResponse(
      engine,
      'invalid_request',
      'That verification token is not valid.',
      400,
      DOCS.invalid_request,
    );
  }
  if (Date.parse(row.expires_at) < nowMs(env)) {
    return errorResponse(
      engine,
      'invalid_request',
      'That verification token has expired. Sign up again.',
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
  return json(200, {
    test_key: testKey,
    live_key: liveKey,
    message:
      'Email verified. Use the test key while you integrate — same engine, unmetered. The live key counts against your plan.',
  });
}

async function checkout(request, env, engine) {
  const auth = await authenticate(request, env, engine);
  if (auth.response) {
    return auth.response;
  }
  let payload;
  try {
    payload = await readBody(request);
  } catch {
    return errorResponse(
      engine,
      'malformed_json',
      'Checkout expects JSON { "plan": "starter" }.',
      400,
      DOCS.malformed_json,
    );
  }
  const planId = String(payload?.plan ?? '');
  if (!PAID_PLANS.has(planId) || !livePlan(planId)) {
    return errorResponse(
      engine,
      'unknown_plan',
      `Unknown plan ${planId || '(empty)'}. Published CAD tiers are starter, growth, and business at ${PLANS.upgrade_url}.`,
      400,
      DOCS.unknown_plan,
    );
  }
  const priceKey = PRICE_ENV[planId];
  const price = env[priceKey];
  if (!price) {
    return errorResponse(
      engine,
      'unknown_plan',
      `Plan ${planId} is published but has no Stripe price configured.`,
      500,
      DOCS.unknown_plan,
    );
  }
  const success_url =
    payload.success_url ?? `${siteUrl(env)}/signup/verify/?ok=1`;
  const cancel_url = payload.cancel_url ?? `${siteUrl(env)}/pricing/`;
  const stripe = stripeClient(env);
  const session = await Promise.resolve(
    stripe.createCheckoutSession({
      currency: 'cad',
      price,
      client_reference_id: auth.account.id,
      metadata: { plan: planId },
      success_url,
      cancel_url,
      mode: 'subscription',
    }),
  );
  return json(200, { url: session.url, currency: 'cad', plan: planId });
}

async function webhook(request, env, engine) {
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
  const raw = await request.text();
  let event;
  try {
    event = stripeClient(env).constructEvent(
      raw,
      request.headers.get('stripe-signature'),
      env.STRIPE_WEBHOOK_SECRET,
    );
  } catch {
    return errorResponse(
      engine,
      'invalid_request',
      'Stripe webhook payload was not a valid event.',
      400,
      DOCS.invalid_request,
    );
  }
  if (event?.id) {
    const seen = await Promise.resolve(store.hasStripeEvent(event.id));
    if (seen) {
      return json(200, { received: true });
    }
    await Promise.resolve(store.recordStripeEvent(event.id));
  }
  if (event?.type === 'checkout.session.completed') {
    const session = event.data?.object ?? {};
    const accountId = session.client_reference_id;
    const plan = session.metadata?.plan;
    if (accountId && PAID_PLANS.has(plan)) {
      await Promise.resolve(store.setPlan(accountId, plan));
    }
  }
  return json(200, { received: true });
}

async function sendMail(env, message) {
  if (Array.isArray(env.MAILBOX)) {
    env.MAILBOX.push(message);
    return;
  }
  if (typeof env.MAILBOX?.push === 'function') {
    env.MAILBOX.push(message);
  }
}

async function readBody(request) {
  const type = request.headers.get('content-type') ?? '';
  const raw = await request.text();
  if (type.includes('application/x-www-form-urlencoded')) {
    return Object.fromEntries(new URLSearchParams(raw));
  }
  if (!raw) {
    return {};
  }
  return JSON.parse(raw);
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
  return json(200, {
    status: 'ok',
    rule_set_version: parsed.rule_set_version,
    engine_version: parsed.engine_version,
    engine_build_sha256: parsed.engine_build_sha256,
  });
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
  });
}
