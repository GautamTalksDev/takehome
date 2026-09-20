import { redactSecrets } from './keys.js';
import { clientIp } from './rate-limit.js';
import { MailTransportError, sendMail } from './mail.js';

export { redactSecrets };

const SALARY_KEYS = new Set(['gross_pay']);

const ALERTABLE = new Set([
  'auth_failures',
  'quota_402',
  'webhook_delivery_failed',
  'rules_watch_drift',
  'http_5xx',
  'engine_error',
]);

const EVENT_KEYS = new Set([
  'type',
  'status',
  'path',
  'ip',
  'count',
  'reason',
  'endpoint_id',
  'last_http_status',
  'last_error',
  'id',
]);

/** Failures from one IP in the window before an auth_failures alert. */
export const AUTH_FAIL_ALERT_AFTER = 5;

const authFails = new Map();

export class UnsafeLogError extends Error {
  constructor() {
    super('refused to log an object that contains a salary field');
    this.name = 'UnsafeLogError';
  }
}

export function resetSecurityLog() {
  authFails.clear();
}

/**
 * A09:2025 item 48. Salary figures must not enter the log pipeline,
 * including nested objects and stringified request bodies.
 */
export function assertLoggable(value, seen = new Set()) {
  if (value == null) {
    return;
  }
  if (typeof value === 'string') {
    if (/gross_pay/i.test(value)) {
      throw new UnsafeLogError();
    }
    return;
  }
  if (typeof value !== 'object') {
    return;
  }
  if (seen.has(value)) {
    return;
  }
  seen.add(value);
  if (Array.isArray(value)) {
    for (const item of value) {
      assertLoggable(item, seen);
    }
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    if (SALARY_KEYS.has(key) || SALARY_KEYS.has(String(key).toLowerCase())) {
      throw new UnsafeLogError();
    }
    assertLoggable(child, seen);
  }
}

/**
 * A10:2025 item 52. Full exception detail for operators. Secrets and
 * salary field names are stripped; the client never sees this payload.
 */
export function logException(err, type = 'exception') {
  const name = err?.name ? String(err.name) : 'Error';
  const message = redactSecrets(String(err?.message ?? err ?? '')).replace(
    /gross_pay/gi,
    '[redacted-field]',
  );
  const stack = redactSecrets(String(err?.stack ?? '')).replace(
    /gross_pay/gi,
    '[redacted-field]',
  );
  console.log(JSON.stringify({ type, name, message, stack }));
}

/** The only console wrapper in this crate. Arguments are redacted first. */
export function log(...args) {
  for (const arg of args) {
    assertLoggable(arg);
  }
  console.log(
    ...args.map((arg) =>
      typeof arg === 'string' ? redactSecrets(arg) : redactSecrets(JSON.stringify(arg)),
    ),
  );
}

function pathOnly(request) {
  try {
    return new URL(request.url).pathname;
  } catch {
    return null;
  }
}

function sanitizeEvent(event, request) {
  const out = {};
  for (const [key, value] of Object.entries(event ?? {})) {
    if (!EVENT_KEYS.has(key)) {
      continue;
    }
    if (
      value == null ||
      typeof value === 'string' ||
      typeof value === 'number' ||
      typeof value === 'boolean'
    ) {
      out[key] = value;
    } else {
      out[key] = String(value);
    }
  }
  if (request) {
    out.path = pathOnly(request);
    out.ip = clientIp(request);
  }
  return out;
}

/**
 * A09:2025 item 50. Push to the in-process ALERTS sink and, when set,
 * email ALERT_EMAIL through the same sendMail path signup uses (MAILBOX
 * in tests, Resend in production). No outbound webhook — that would be SSRF.
 * Alert mail failures are logged; they must not change the HTTP response.
 */
export async function dispatchAlert(env, event) {
  assertLoggable(event);
  if (Array.isArray(env.ALERTS)) {
    env.ALERTS.push({ ...event, alerted: true });
  }
  const to = env.ALERT_EMAIL;
  if (!to) {
    return;
  }
  try {
    await sendMail(env, {
      to: String(to),
      subject: `Takehome alert: ${event.type}`,
      text: JSON.stringify(event),
    });
  } catch (err) {
    if (err instanceof MailTransportError) {
      log({ type: 'alert_mail_failed', reason: err.message });
      return;
    }
    throw err;
  }
}

/**
 * A09:2025 items 49–50. Structured security event. Request bodies are
 * never attached. Alertable types also reach ALERTS / ALERT_EMAIL.
 */
export async function recordSecurityEvent(env, event, request) {
  const safeEvent = sanitizeEvent(event, request);
  assertLoggable(safeEvent);
  log(safeEvent);
  if (ALERTABLE.has(safeEvent.type)) {
    await dispatchAlert(env, safeEvent);
  }
}

/**
 * A09:2025 item 49. Count 401s per IP; alert once when the window hits
 * AUTH_FAIL_ALERT_AFTER.
 */
export async function noteAuthFailure(request, env) {
  const ip = clientIp(request);
  const windowMs = Number.parseInt(String(env.RATE_LIMIT_WINDOW_MS ?? '60000'), 10);
  const now = Date.now();
  const previous = (authFails.get(ip) ?? []).filter((stamp) => now - stamp < windowMs);
  previous.push(now);
  authFails.set(ip, previous);
  if (previous.length === AUTH_FAIL_ALERT_AFTER) {
    await recordSecurityEvent(
      env,
      { type: 'auth_failures', count: previous.length },
      request,
    );
  }
}

/**
 * A09:2025 item 49. Central response observer: auth bursts, quota 402s,
 * and any 5xx (including untyped engine errors). Typed 4xx is silent.
 */
export async function observeResponse(request, env, response) {
  const status = response.status;
  if (status === 401) {
    await noteAuthFailure(request, env);
    return;
  }
  if (status === 402) {
    await recordSecurityEvent(env, { type: 'quota_402', status: 402 }, request);
    return;
  }
  if (status >= 500) {
    await recordSecurityEvent(env, { type: 'http_5xx', status }, request);
  }
}
