/**
 * CORS is open only for public, unauthenticated listings.
 * Keyed routes (/v1/deductions, /v1/webhooks, signup, billing) are same-origin
 * or server-side; Access-Control-Allow-Origin: * on those invites live keys
 * in front-end code.
 */

export function isPublicCorsPath(path) {
  return (
    path === '/openapi.json' ||
    path === '/v1/conformance' ||
    path === '/v1/changes' ||
    path === '/v1/changes.rss' ||
    path === '/v1/rules' ||
    path.startsWith('/v1/rules/')
  );
}

export function publicCorsHeaders() {
  return {
    'access-control-allow-origin': '*',
    'access-control-allow-methods': 'GET, OPTIONS',
    'access-control-allow-headers': 'content-type',
  };
}
