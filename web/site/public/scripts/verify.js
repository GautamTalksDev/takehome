import { apiOrigin, displayOrigin } from './api-origin.js';

const firstCallBody = [
  '{',
  '  "as_of": "2026-01-15",',
  '  "province": "ON",',
  '  "pay_period": 52,',
  '  "gross_pay": "1000.00",',
  '  "federal_claim_code": 1,',
  '  "provincial_claim_code": 1',
  '}',
].join('\n');

const status = document.getElementById('verify-status');
const keysPanel = document.getElementById('verify-keys-panel');
const testKeyEl = document.getElementById('verify-test-key');
const liveKeyEl = document.getElementById('verify-live-key');
const hint = document.getElementById('verify-hint');
const curlWrap = document.getElementById('verify-curl-wrap');
const curl = document.getElementById('verify-curl');

function firstCallCurl(testKey) {
  const origin = displayOrigin();
  return [
    `curl -sS -X POST \\`,
    `  ${origin}/v1/deductions \\`,
    `  -H 'content-type: application/json' \\`,
    `  -H 'authorization: Bearer ${testKey}' \\`,
    `  -d '${firstCallBody}'`,
  ].join('\n');
}

if (status && keysPanel && testKeyEl && liveKeyEl && hint && curlWrap && curl) {
  const token = new URLSearchParams(location.search).get('token');
  if (!token) {
    status.textContent =
      'Open the link from your email. It includes a one-time token.';
  } else {
    fetch(`${apiOrigin()}/v1/signup/verify?token=${encodeURIComponent(token)}`)
      .then(async (response) => {
        const body = await response.json();
        if (!response.ok) {
          status.textContent = body.error?.message ?? 'Verification failed.';
          return;
        }
        status.textContent = body.message;
        testKeyEl.textContent = body.test_key;
        liveKeyEl.textContent = body.live_key;
        keysPanel.hidden = false;
        hint.hidden = false;
        curl.textContent = firstCallCurl(body.test_key);
        curlWrap.hidden = false;
      })
      .catch(() => {
        status.textContent = 'Could not reach the API. Try the link again.';
      });
  }
}
