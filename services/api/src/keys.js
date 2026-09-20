import { createHash, timingSafeEqual } from 'node:crypto';

/** 128 bits. API keys use this many CSPRNG bytes after the prefix. */
export const KEY_RANDOM_BYTES = 16;
/** 128 bits. Email verification tokens. */
export const TOKEN_RANDOM_BYTES = 16;
/** 256 bits. Webhook HMAC secrets (`whsec_`). */
export const WEBHOOK_SECRET_BYTES = 32;

/**
 * Lowercase hex from `crypto.getRandomValues`. Hex is unambiguous
 * (no 0/O, 1/l/I). Do not draw key material from the wall clock or a
 * non-CSPRNG. Keep the alphabet strict so glyphs stay unconfusable.
 */
export function randomHex(bytes) {
  const buf = new Uint8Array(bytes);
  crypto.getRandomValues(buf);
  let out = '';
  for (const octet of buf) {
    out += octet.toString(16).padStart(2, '0');
  }
  return out;
}

/**
 * SHA-256 hex of an API key (or other high-entropy token).
 * See docs/ADR-004-api-key-hash.md — this is not a password hash.
 */
export function hashKey(secret) {
  return createHash('sha256').update(secret).digest('hex');
}

/**
 * Constant-time equality for digests and HMAC tags.
 * A non-constant-time compare on an HMAC is a signature-forgery oracle.
 */
export function equalDigest(a, b) {
  const left = Buffer.isBuffer(a) ? a : Buffer.from(String(a));
  const right = Buffer.isBuffer(b) ? b : Buffer.from(String(b));
  if (left.length !== right.length) {
    return false;
  }
  return timingSafeEqual(left, right);
}

export const WEBHOOK_ID_PATTERN = /^wh_[0-9a-f]{32}$/;
export const DELIVERY_ID_PATTERN = /^whd_[0-9a-f]{32}$/;

/** Dummy digest so a miss still runs `equalDigest` (A04 item 29). */
export const DIGEST_PLACEHOLDER = hashKey('equal-digest-placeholder');

export function newWebhookId() {
  return `wh_${randomHex(16)}`;
}

export function newDeliveryId() {
  return `whd_${randomHex(16)}`;
}

export function mintKey(kind) {
  if (kind !== 'test' && kind !== 'live') {
    throw new Error(`unknown key kind ${kind}`);
  }
  return `np_${kind}_${randomHex(KEY_RANDOM_BYTES)}`;
}

export function mintWebhookSecret() {
  return `whsec_${randomHex(WEBHOOK_SECRET_BYTES)}`;
}

export function mintVerifyToken() {
  return randomHex(TOKEN_RANDOM_BYTES);
}

export const KEY_HEX_CHARS = KEY_RANDOM_BYTES * 2;

export const KEY_PATTERN = new RegExp(
  `^np_(test|live)_([0-9a-f]{${KEY_HEX_CHARS}})$`,
);

/** Hashed on invalid-format paths so lookup still runs (A07 item 41). */
export const AUTH_DUMMY_SECRET = `np_live_${'0'.repeat(KEY_HEX_CHARS)}`;

export function parseBearer(request) {
  const header = request.headers.get('authorization');
  if (!header) {
    return null;
  }
  const match = header.match(/^Bearer\s+(\S+)$/i);
  return match ? match[1] : null;
}

export function keyKind(secret) {
  const match = String(secret ?? '').match(KEY_PATTERN);
  return match ? match[1] : null;
}

/** Strip API keys and webhook secrets, including truncated hex bodies. */
export function redactSecrets(value) {
  return String(value)
    .replace(/np_(?:test|live)_[0-9a-f]+/gi, '[redacted]')
    .replace(/whsec_[0-9a-f]+/gi, '[redacted]');
}

export function issueKeyPair(store, accountId) {
  const testKey = mintKey('test');
  const liveKey = mintKey('live');
  store.insertKey({
    account_id: accountId,
    kind: 'test',
    prefix: 'np_test_',
    hash: hashKey(testKey),
  });
  store.insertKey({
    account_id: accountId,
    kind: 'live',
    prefix: 'np_live_',
    hash: hashKey(liveKey),
  });
  return { testKey, liveKey };
}
