const hits = new Map();

const BUCKETS = {
  public: { maxEnv: 'RATE_LIMIT_MAX', defaultMax: 120 },
  signup: { maxEnv: 'SIGNUP_RATE_LIMIT_MAX', defaultMax: 5 },
  verify: { maxEnv: 'VERIFY_RATE_LIMIT_MAX', defaultMax: 10 },
};

export function resetRateLimits() {
  hits.clear();
}

export function clientIp(request) {
  const cf = request.headers.get('cf-connecting-ip');
  if (cf) {
    return cf;
  }
  const forwarded = request.headers.get('x-forwarded-for');
  if (forwarded) {
    return forwarded.split(',')[0].trim();
  }
  return 'local';
}

/**
 * Unauthenticated routes share an IP bucket. Signup and verify are
 * separate, tighter buckets (A06:2025). Keyed routes are metered by plan,
 * not this limiter.
 */
export function rateBucket(path, method) {
  if (method === 'OPTIONS') {
    return null;
  }
  if (path === '/v1/signup' && method === 'POST') {
    return 'signup';
  }
  if (path === '/v1/signup/verify' && method === 'GET') {
    return 'verify';
  }
  if (
    path === '/health' ||
    path === '/openapi.json' ||
    path === '/v1/jurisdictions' ||
    path === '/v1/conformance' ||
    path === '/v1/changes' ||
    path === '/v1/changes.rss' ||
    path === '/v1/rules' ||
    path.startsWith('/v1/rules/') ||
    path === '/v1/billing/webhook'
  ) {
    return 'public';
  }
  return null;
}

export function checkRateLimit(request, env, bucket = 'public') {
  const spec = BUCKETS[bucket] ?? BUCKETS.public;
  const max = Number.parseInt(
    String(env[spec.maxEnv] ?? spec.defaultMax),
    10,
  );
  const windowMs = Number.parseInt(
    String(env.RATE_LIMIT_WINDOW_MS ?? '60000'),
    10,
  );
  const ip = clientIp(request);
  const key = `${bucket}:${ip}`;
  const now = Date.now();
  const previous = hits.get(key) ?? [];
  const fresh = previous.filter((stamp) => now - stamp < windowMs);
  if (fresh.length >= max) {
    hits.set(key, fresh);
    return { allowed: false, ip, remaining: 0, limit: max, bucket };
  }
  fresh.push(now);
  hits.set(key, fresh);
  return {
    allowed: true,
    ip,
    remaining: max - fresh.length,
    limit: max,
    bucket,
  };
}
