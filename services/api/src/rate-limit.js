const hits = new Map();

function clientIp(request) {
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

export function checkRateLimit(request, env) {
  const max = Number.parseInt(String(env.RATE_LIMIT_MAX ?? '120'), 10);
  const windowMs = Number.parseInt(
    String(env.RATE_LIMIT_WINDOW_MS ?? '60000'),
    10,
  );
  const ip = clientIp(request);
  const now = Date.now();
  const previous = hits.get(ip) ?? [];
  const fresh = previous.filter((stamp) => now - stamp < windowMs);
  if (fresh.length >= max) {
    hits.set(ip, fresh);
    return { allowed: false, ip, remaining: 0, limit: max };
  }
  fresh.push(now);
  hits.set(ip, fresh);
  return {
    allowed: true,
    ip,
    remaining: max - fresh.length,
    limit: max,
  };
}
