/**
 * API origin for browser pages. Prefer data-api (local PUBLIC_API_ORIGIN).
 * Otherwise same-origin relative URLs so CSP connect-src 'self' works on
 * the production host (site and Worker share the origin).
 */
export function apiOrigin() {
  const fromDoc = document.documentElement.getAttribute('data-api');
  if (fromDoc) return fromDoc;
  return '';
}

/** Absolute origin for copy-paste curls on the verify page. */
export function displayOrigin() {
  const api = apiOrigin();
  if (api) return api;
  return location.origin;
}
