import { apiOrigin, displayOrigin } from './api-origin.js';

const firstCallJson =
  '{"as_of":"2026-01-15","province":"ON","pay_period":52,"gross_pay":"1000.00","federal_claim_code":1,"provincial_claim_code":1}';
const status = document.getElementById('verify-status');
const keys = document.getElementById('verify-keys');
const hint = document.getElementById('verify-hint');
const curl = document.getElementById('verify-curl');

if (status && keys && hint && curl) {
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
        keys.hidden = false;
        keys.textContent = `test: ${body.test_key}\nlive: ${body.live_key}`;
        hint.hidden = false;
        curl.hidden = false;
        curl.textContent =
          `curl -sS -X POST ${displayOrigin()}/v1/deductions \\\n` +
          `  -H 'content-type: application/json' \\\n` +
          `  -H 'authorization: Bearer ${body.test_key}' \\\n` +
          `  -d '${firstCallJson}'`;
      })
      .catch(() => {
        status.textContent = 'Could not reach the API. Try the link again.';
      });
  }
}
