/**
 * Outbound rule-set webhooks (spec §12.3).
 *
 * HMAC-SHA256 over `timestamp.body`, Stripe-style header
 * `takehome-signature: t=<unix>,v1=<hex>`. Retries with exponential backoff.
 * The sleep function is injected so tests never wait on a wall clock.
 */

import { createHmac, timingSafeEqual } from 'node:crypto';

export const SIGNATURE_HEADER = 'takehome-signature';
export const BACKOFF_MS = Object.freeze([1000, 2000, 4000]);
export const MAX_ATTEMPTS = 3;
export const TOLERANCE_SEC = 300;

export function signPayload(secret, body, timestampSec) {
  const t = String(timestampSec);
  const mac = createHmac('sha256', secret).update(`${t}.${body}`).digest('hex');
  return `t=${t},v1=${mac}`;
}

export function verifySignature(secret, body, header, nowSec, toleranceSec = TOLERANCE_SEC) {
  if (!secret || !header) {
    return false;
  }
  const parsed = parseSignature(header);
  if (!parsed) {
    return false;
  }
  if (Math.abs(nowSec - parsed.t) > toleranceSec) {
    return false;
  }
  const expected = signPayload(secret, body, parsed.t);
  const expectedMac = signatureMac(expected);
  const gotMac = signatureMac(header);
  if (!expectedMac || !gotMac || expectedMac.length !== gotMac.length) {
    return false;
  }
  return timingSafeEqual(expectedMac, gotMac);
}

function parseSignature(header) {
  const parts = Object.fromEntries(
    String(header)
      .split(',')
      .map((part) => part.trim().split('='))
      .filter((pair) => pair.length === 2),
  );
  const t = Number.parseInt(parts.t, 10);
  if (!Number.isFinite(t) || !parts.v1) {
    return null;
  }
  return { t, v1: parts.v1 };
}

function signatureMac(header) {
  const parsed = parseSignature(header);
  if (!parsed) {
    return null;
  }
  return Buffer.from(parsed.v1, 'hex');
}

export async function deliverWithRetry({
  url,
  body,
  secret,
  timestampSec,
  fetchImpl,
  sleep,
  maxAttempts = MAX_ATTEMPTS,
  backoffMs = BACKOFF_MS,
}) {
  let lastStatus = null;
  let lastError = null;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    if (attempt > 1) {
      const delay = backoffMs[attempt - 2] ?? backoffMs[backoffMs.length - 1];
      await sleep(delay);
    }
    const signature = signPayload(secret, body, timestampSec);
    try {
      const response = await fetchImpl(url, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          [SIGNATURE_HEADER]: signature,
        },
        body,
      });
      lastStatus = response.status;
      if (response.ok) {
        return {
          ok: true,
          attempts: attempt,
          status: response.status,
          error: null,
        };
      }
      lastError = `HTTP ${response.status}`;
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
    }
  }
  return {
    ok: false,
    attempts: maxAttempts,
    status: lastStatus,
    error: lastError,
  };
}

export function webhookPayload({ from, to, changed, occurredAt }) {
  return JSON.stringify({
    event: 'rule_set.changed',
    from,
    to,
    changed,
    occurred_at: occurredAt,
  });
}
