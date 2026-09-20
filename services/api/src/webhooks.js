/**
 * Outbound rule-set webhooks (spec §12.3).
 *
 * HMAC-SHA256 over `timestamp.body` (never the body alone), Stripe-style
 * header `takehome-signature: t=<unix>,v1=<hex>`. Receivers must reject
 * `|now - t| > 300` seconds (5 minutes) — that is the replay window.
 * Delivery uses redirect: "manual", an AbortSignal timeout, and a capped
 * body read; the response body is discarded, never stored. The sleep
 * function is injected so tests never wait on a wall clock.
 */

import { createHmac } from 'node:crypto';
import { equalDigest } from './keys.js';

export const SIGNATURE_HEADER = 'takehome-signature';
export const BACKOFF_MS = Object.freeze([1000, 2000, 4000]);
export const MAX_ATTEMPTS = 3;
export const TOLERANCE_SEC = 300;
export const DELIVERY_TIMEOUT_MS = 5000;
export const MAX_RESPONSE_BODY_BYTES = 4096;

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
  return equalDigest(expectedMac, gotMac);
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

function isAbortError(err) {
  if (!err || typeof err !== 'object') {
    return false;
  }
  return err.name === 'AbortError' || err.code === 'ABORT_ERR';
}

function isRedirectStatus(status) {
  return status >= 300 && status < 400;
}

async function discardBodyCapped(response, maxBytes) {
  if (!response || !response.body || typeof response.body.getReader !== 'function') {
    return;
  }
  const reader = response.body.getReader();
  let read = 0;
  try {
    while (read < maxBytes) {
      const { done, value } = await reader.read();
      if (done) {
        return;
      }
      read += value ? value.byteLength : 0;
    }
  } finally {
    try {
      await reader.cancel();
    } catch {
      // The stream is attacker-controlled; drop it either way.
    }
  }
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
  timeoutMs = DELIVERY_TIMEOUT_MS,
  maxBodyBytes = MAX_RESPONSE_BODY_BYTES,
}) {
  let lastStatus = null;
  let lastError = null;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    if (attempt > 1) {
      const delay = backoffMs[attempt - 2] ?? backoffMs[backoffMs.length - 1];
      await sleep(delay);
    }
    const signature = signPayload(secret, body, timestampSec);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetchImpl(url, {
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          [SIGNATURE_HEADER]: signature,
        },
        body,
        redirect: 'manual',
        signal: controller.signal,
      });
      lastStatus = response.status;
      await discardBodyCapped(response, maxBodyBytes);
      if (response.ok) {
        return {
          ok: true,
          attempts: attempt,
          status: response.status,
          error: null,
        };
      }
      lastError = `HTTP ${response.status}`;
      if (isRedirectStatus(response.status)) {
        break;
      }
    } catch (err) {
      if (isAbortError(err) || controller.signal.aborted) {
        lastError = 'webhook delivery timed out';
        break;
      }
      lastError = err instanceof Error ? err.message : String(err);
    } finally {
      clearTimeout(timer);
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
