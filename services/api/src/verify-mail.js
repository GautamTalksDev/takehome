/**
 * Verification email bodies. Plain text plus minimal HTML. No images,
 * no tracking. The verify URL appears exactly once in each version.
 */

export function verificationEmailBodies({ verifyUrl, expiresAt }) {
  const url = String(verifyUrl);
  const expiry = String(expiresAt);
  const text = [
    'Takehome calculates Canadian payroll deductions from the CRA T4127 formulas.',
    '',
    'Click the link to verify your email. You receive a test key (unmetered) and a live key.',
    '',
    url,
    '',
    `This link works once and expires at ${expiry} UTC (24 hours from issue).`,
    '',
    'If your client strips links, copy the URL on the line above into your browser.',
  ].join('\n');

  const html = [
    '<p>Takehome calculates Canadian payroll deductions from the CRA T4127 formulas.</p>',
    '<p>Click the link to verify your email. You receive a test key (unmetered) and a live key.</p>',
    `<p><a href="${escapeAttr(url)}">Verify your email</a></p>`,
    `<p>This link works once and expires at ${escapeHtml(expiry)} UTC (24 hours from issue).</p>`,
    '<p>If your client strips links, copy the URL from your plain-text copy of this message.</p>',
  ].join('\n');

  return { text, html };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", '&#39;');
}
