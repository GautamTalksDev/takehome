import { apiOrigin } from './api-origin.js';

const form = document.getElementById('signup-form');
const status = document.getElementById('signup-status');
const wrap = document.getElementById('signup-verify-wrap');
const link = document.getElementById('signup-verify');

if (form && status && wrap && link) {
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    status.textContent = 'Sending…';
    wrap.hidden = true;
    const email = new FormData(form).get('email');
    try {
      const response = await fetch(`${apiOrigin()}/v1/signup`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email }),
      });
      const body = await response.json();
      status.textContent =
        body.message ?? body.error?.message ?? 'Check your email.';
      if (body.verify_url) {
        status.textContent =
          'Open the verification link. Keys are on the next page.';
        link.href = body.verify_url;
        wrap.hidden = false;
      }
    } catch {
      status.textContent = 'Could not reach the API. Try again.';
    }
  });
}
