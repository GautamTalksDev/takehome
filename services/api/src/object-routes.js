/**
 * Object-ID routes. Authorization is per-account and a miss is 404, never 403.
 * The handler dispatches from this table; tests loop it so a new ID-taking
 * route cannot skip the BOLA check.
 */

const WEBHOOK_RESERVED = new Set(['dispatch', 'deliveries']);

function webhookIdMatch(pathname) {
  const matched = pathname.match(/^\/v1\/webhooks\/([^/]+)$/);
  if (!matched) {
    return null;
  }
  const id = decodeURIComponent(matched[1]);
  if (WEBHOOK_RESERVED.has(id)) {
    return null;
  }
  return { id };
}

function deliveryReplayMatch(pathname) {
  const matched = pathname.match(
    /^\/v1\/webhooks\/deliveries\/([^/]+)\/replay$/,
  );
  if (!matched) {
    return null;
  }
  return { id: decodeURIComponent(matched[1]) };
}

export const OBJECT_ID_ROUTES = Object.freeze([
  Object.freeze({
    name: 'getWebhook',
    method: 'GET',
    pattern: '/v1/webhooks/:id',
    idKind: 'webhook',
    path: (id) => `/v1/webhooks/${encodeURIComponent(id)}`,
    match: webhookIdMatch,
  }),
  Object.freeze({
    name: 'deleteWebhook',
    method: 'DELETE',
    pattern: '/v1/webhooks/:id',
    idKind: 'webhook',
    path: (id) => `/v1/webhooks/${encodeURIComponent(id)}`,
    match: webhookIdMatch,
  }),
  Object.freeze({
    name: 'replayDelivery',
    method: 'POST',
    pattern: '/v1/webhooks/deliveries/:id/replay',
    idKind: 'delivery',
    path: (id) => `/v1/webhooks/deliveries/${encodeURIComponent(id)}/replay`,
    match: deliveryReplayMatch,
  }),
]);

export function matchObjectIdRoute(method, pathname) {
  for (const route of OBJECT_ID_ROUTES) {
    if (route.method !== method) {
      continue;
    }
    const matched = route.match(pathname);
    if (matched) {
      return { route, id: matched.id };
    }
  }
  return null;
}

export function objectIdMethodsFor(pathname) {
  const methods = [];
  for (const route of OBJECT_ID_ROUTES) {
    if (route.match(pathname)) {
      methods.push(route.method);
    }
  }
  return methods;
}
