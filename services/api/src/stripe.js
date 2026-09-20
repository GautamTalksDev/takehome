/**
 * Stripe helpers kept warm while paid billing is deferred (ADR-006).
 * Live routes in handler.js must not import this module until checkout
 * and the webhook are turned back on with real products and prices.
 */
import { createHmac } from 'node:crypto';
import { equalDigest } from './keys.js';

/** Stripe replay window. Same 5-minute bound as customer webhooks. */
export const STRIPE_TOLERANCE_SEC = 300;

/**
 * A10:2025 item 55. Stripe-compatible `t=<unix>,v1=<hex>` over
 * `timestamp.payload`. HMAC-SHA256, compared with equalDigest.
 */
export function signStripeEvent(secret, raw, timestampSec) {
  const t = String(timestampSec);
  const mac = createHmac('sha256', secret).update(`${t}.${raw}`).digest('hex');
  return `t=${t},v1=${mac}`;
}

export function verifyStripeSignature(
  secret,
  raw,
  header,
  nowSec,
  toleranceSec = STRIPE_TOLERANCE_SEC,
) {
  if (!secret || !header) {
    return false;
  }
  const parts = Object.fromEntries(
    String(header)
      .split(',')
      .map((part) => part.trim().split('='))
      .filter((pair) => pair.length === 2),
  );
  const t = Number.parseInt(parts.t, 10);
  if (!Number.isFinite(t) || !parts.v1) {
    return false;
  }
  if (Math.abs(nowSec - t) > toleranceSec) {
    return false;
  }
  const expected = createHmac('sha256', secret)
    .update(`${t}.${raw}`)
    .digest('hex');
  const got = String(parts.v1);
  if (got.length !== expected.length) {
    return false;
  }
  return equalDigest(Buffer.from(got, 'hex'), Buffer.from(expected, 'hex'));
}

export function constructStripeEvent(raw, header, secret, nowSec) {
  if (!secret) {
    throw new Error('missing stripe webhook secret');
  }
  if (!verifyStripeSignature(secret, raw, header, nowSec)) {
    throw new Error('invalid stripe signature');
  }
  return JSON.parse(raw);
}

export function stripeClient(env) {
  if (env.STRIPE) {
    return env.STRIPE;
  }
  return {
    async createCheckoutSession(input) {
      const body = new URLSearchParams();
      body.set('mode', input.mode ?? 'subscription');
      body.set('success_url', input.success_url);
      body.set('cancel_url', input.cancel_url);
      body.set('client_reference_id', input.client_reference_id);
      body.set('metadata[plan]', input.metadata.plan);
      body.set('line_items[0][price]', input.price);
      body.set('line_items[0][quantity]', '1');
      const response = await fetch(
        'https://api.stripe.com/v1/checkout/sessions',
        {
          method: 'POST',
          headers: {
            authorization: `Bearer ${env.STRIPE_SECRET_KEY}`,
            'content-type': 'application/x-www-form-urlencoded',
          },
          body,
        },
      );
      const json = await response.json();
      if (!response.ok) {
        throw new Error(json.error?.message ?? 'stripe_checkout_failed');
      }
      return {
        ...json,
        currency: json.currency ?? 'cad',
        price: input.price,
      };
    },
    constructEvent(raw, header, secret, nowSec) {
      return constructStripeEvent(raw, header, secret, nowSec);
    },
  };
}
