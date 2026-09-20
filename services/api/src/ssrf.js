/**
 * Webhook URL SSRF controls (OWASP A01:2025 / API7:2023).
 *
 * Registration and dispatch both resolve the hostname and refuse private,
 * link-local, and other non-public addresses. Literal IP hostnames (decimal,
 * octal, hex, IPv6) are refused before DNS. Dispatch re-validates immediately
 * before fetch so a DNS rebinding between register and deliver cannot pin
 * the Worker to loopback or cloud metadata.
 */

export class UnsafeWebhookUrlError extends Error {
  constructor(message) {
    super(message);
    this.name = 'UnsafeWebhookUrlError';
    this.code = 'unsafe_webhook_url';
  }
}

const V4_BLOCKS = Object.freeze([
  [ipv4(127, 0, 0, 0), 8],
  [ipv4(10, 0, 0, 0), 8],
  [ipv4(172, 16, 0, 0), 12],
  [ipv4(192, 168, 0, 0), 16],
  [ipv4(169, 254, 0, 0), 16],
  [ipv4(0, 0, 0, 0), 8],
  [ipv4(100, 64, 0, 0), 10],
  [ipv4(192, 0, 0, 0), 24],
  [ipv4(198, 18, 0, 0), 15],
  [ipv4(224, 0, 0, 0), 4],
  [ipv4(240, 0, 0, 0), 4],
]);

const BLOCKED_SINGLE_LABELS = new Set(['localhost', 'internal', 'local']);

function ipv4(a, b, c, d) {
  return ((a << 24) | (b << 16) | (c << 8) | d) >>> 0;
}

function ipv4InCidr(ip, base, prefix) {
  const shift = 32 - prefix;
  const mask = prefix === 0 ? 0 : (0xffffffff << shift) >>> 0;
  return (ip & mask) === (base & mask);
}

function parseIpv4Part(part) {
  if (!part) {
    return null;
  }
  if (/^0x[0-9a-f]+$/i.test(part)) {
    const n = Number.parseInt(part.slice(2), 16);
    return Number.isInteger(n) ? n : null;
  }
  if (/^0[0-7]+$/.test(part)) {
    return Number.parseInt(part, 8);
  }
  if (/^[0-9]+$/.test(part)) {
    return Number.parseInt(part, 10);
  }
  return null;
}

function combineInetAton(nums) {
  switch (nums.length) {
    case 1:
      if (nums[0] > 0xffffffff) {
        return null;
      }
      return nums[0] >>> 0;
    case 2:
      if (nums[0] > 0xff || nums[1] > 0xffffff) {
        return null;
      }
      return ((nums[0] << 24) | nums[1]) >>> 0;
    case 3:
      if (nums[0] > 0xff || nums[1] > 0xff || nums[2] > 0xffff) {
        return null;
      }
      return ((nums[0] << 24) | (nums[1] << 16) | nums[2]) >>> 0;
    case 4:
      if (nums.some((n) => n > 0xff)) {
        return null;
      }
      return ((nums[0] << 24) | (nums[1] << 16) | (nums[2] << 8) | nums[3]) >>> 0;
    default:
      return null;
  }
}

function parseIPv4Encoded(host) {
  const parts = host.split('.');
  if (parts.length < 1 || parts.length > 4) {
    return null;
  }
  const nums = [];
  for (const part of parts) {
    const n = parseIpv4Part(part);
    if (n === null) {
      return null;
    }
    nums.push(n);
  }
  return combineInetAton(nums);
}

function parseIPv4DottedDecimal(host) {
  const parts = host.split('.');
  if (parts.length !== 4) {
    return null;
  }
  const nums = [];
  for (const part of parts) {
    if (!/^[0-9]{1,3}$/.test(part)) {
      return null;
    }
    const n = Number.parseInt(part, 10);
    if (n > 255) {
      return null;
    }
    nums.push(n);
  }
  return combineInetAton(nums);
}

function parseIPv6(input) {
  let s = String(input).toLowerCase();
  if (s.startsWith('[') && s.endsWith(']')) {
    s = s.slice(1, -1);
  }
  const pct = s.indexOf('%');
  if (pct !== -1) {
    s = s.slice(0, pct);
  }
  const lastColon = s.lastIndexOf(':');
  if (lastColon !== -1 && s.includes('.')) {
    const tail = s.slice(lastColon + 1);
    if (tail.includes('.')) {
      const v4 = parseIPv4DottedDecimal(tail);
      if (v4 === null) {
        return null;
      }
      const hi = (v4 >>> 16) & 0xffff;
      const lo = v4 & 0xffff;
      s = `${s.slice(0, lastColon + 1)}${hi.toString(16)}:${lo.toString(16)}`;
    }
  }
  if (s === '::') {
    return [0, 0, 0, 0, 0, 0, 0, 0];
  }
  let parts;
  if (s.includes('::')) {
    if (s.indexOf('::') !== s.lastIndexOf('::')) {
      return null;
    }
    const [left, right] = s.split('::');
    const leftParts = left === '' ? [] : left.split(':');
    const rightParts = right === '' ? [] : right.split(':');
    if (leftParts.length + rightParts.length > 7) {
      return null;
    }
    const fill = 8 - leftParts.length - rightParts.length;
    parts = [...leftParts, ...Array(fill).fill('0'), ...rightParts];
  } else {
    parts = s.split(':');
  }
  if (parts.length !== 8) {
    return null;
  }
  const hextets = [];
  for (const part of parts) {
    if (!/^[0-9a-f]{1,4}$/.test(part)) {
      return null;
    }
    hextets.push(Number.parseInt(part, 16));
  }
  return hextets;
}

function isBlockedIPv4(ip) {
  return V4_BLOCKS.some(([base, prefix]) => ipv4InCidr(ip, base, prefix));
}

function isBlockedIPv6(hextets) {
  const loopback = hextets.every((x, i) => (i < 7 ? x === 0 : x === 1));
  if (loopback) {
    return true;
  }
  const unspecified = hextets.every((x) => x === 0);
  if (unspecified) {
    return true;
  }
  if ((hextets[0] & 0xfe00) === 0xfc00) {
    return true;
  }
  if ((hextets[0] & 0xffc0) === 0xfe80) {
    return true;
  }
  const mapped =
    hextets[0] === 0 &&
    hextets[1] === 0 &&
    hextets[2] === 0 &&
    hextets[3] === 0 &&
    hextets[4] === 0 &&
    hextets[5] === 0xffff;
  if (mapped) {
    return true;
  }
  return false;
}

export function isBlockedAddress(address) {
  const value = String(address).trim().toLowerCase();
  if (value.includes(':')) {
    const hextets = parseIPv6(value);
    return hextets ? isBlockedIPv6(hextets) : true;
  }
  const ip = parseIPv4Encoded(value);
  if (ip === null) {
    return true;
  }
  return isBlockedIPv4(ip);
}

function normalizeHostname(hostname) {
  return String(hostname).trim().toLowerCase().replace(/\.+$/, '');
}

function isLiteralIpHostname(host) {
  if (host.includes(':')) {
    return parseIPv6(host) !== null;
  }
  return parseIPv4Encoded(host) !== null;
}

function isBlockedHostname(host) {
  if (!host) {
    return true;
  }
  if (!host.includes('.')) {
    return true;
  }
  const labels = host.split('.');
  const last = labels[labels.length - 1];
  if (BLOCKED_SINGLE_LABELS.has(last)) {
    return true;
  }
  return false;
}

export async function lookupHostname(hostname) {
  const dns = await import('node:dns/promises');
  const rows = await dns.lookup(hostname, { all: true, verbatim: true });
  return rows.map((row) => row.address);
}

export async function assertSafeWebhookUrl(urlString, options = {}) {
  let parsed;
  try {
    parsed = new URL(urlString);
  } catch {
    throw new UnsafeWebhookUrlError('Webhook URL is not a valid URL.');
  }
  if (parsed.protocol !== 'https:') {
    throw new UnsafeWebhookUrlError('Webhook URL must be https.');
  }
  const host = normalizeHostname(parsed.hostname);
  if (!host) {
    throw new UnsafeWebhookUrlError('Webhook hostname is not allowed.');
  }
  if (isLiteralIpHostname(host)) {
    throw new UnsafeWebhookUrlError(
      'Webhook hostname must not be an IP address.',
    );
  }
  if (isBlockedHostname(host)) {
    throw new UnsafeWebhookUrlError('Webhook hostname is not allowed.');
  }
  const resolver = options.resolve ?? lookupHostname;
  let addresses;
  try {
    addresses = await resolver(host);
  } catch {
    throw new UnsafeWebhookUrlError('Webhook hostname did not resolve.');
  }
  if (!Array.isArray(addresses) || addresses.length === 0) {
    throw new UnsafeWebhookUrlError('Webhook hostname did not resolve.');
  }
  for (const address of addresses) {
    if (isBlockedAddress(address)) {
      throw new UnsafeWebhookUrlError(
        'Webhook hostname resolved to a blocked address.',
      );
    }
  }
  return parsed;
}
