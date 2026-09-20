# Webhooks

**Who this is for:** Operators who want HTTPS notifications when CRA rule sets change, with verified signatures on inbound POST bodies.

**When you finish:** You can register an endpoint, verify `takehome-signature`, and inspect or replay deliveries from the API.

Takehome sends `rule_set.changed` events when enacted T4127 (Payroll Deductions Formulas) bundles update. Stripe billing webhooks are separate at `POST /v1/billing/webhook`.

<figure>
  <img src="/diagrams/07-webhook-ssrf-guard.svg" alt="Webhook delivery with the SSRF guard" />
  <figcaption>Registration validates the URL. Dispatch re-validates immediately before fetch because DNS can change. Redirects are not followed. Response bodies are never written into the delivery log.</figcaption>
</figure>

## Register an endpoint

```bash runnable
curl -sS -X POST https://takehome.gautamkhosla.com/v1/webhooks \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer np_live_YOUR_KEY_HERE' \
  -d '{"url":"https://your-app.example.com/takehome/webhook"}'
```

The response includes `id`, `url`, `created_at`, and `secret` (`whsec_…`). The secret is shown once. Store it like an API key.

List endpoints: `GET /v1/webhooks`. Delete: `DELETE /v1/webhooks/{id}`.

## Payload

Outbound POST body (JSON):

```json
{
  "event": "rule_set.changed",
  "from": "2026-01-01",
  "to": "2026-07-01",
  "changed": ["cpp", "ei"],
  "occurred_at": "2026-06-15T12:00:00Z"
}
```

Headers include `content-type: application/json` and `takehome-signature`.

## HMAC signature (exact bytes)

Signature header format: `takehome-signature: t=<unix>,v1=<hex>`.

The MAC is HMAC-SHA256 over the UTF-8 string `timestamp + "." + raw_body` where:

- `timestamp` is the same decimal Unix seconds as `t=` (string of digits).
- `raw_body` is the exact HTTP request body bytes (before JSON parsing).

The secret is the `whsec_` value from registration. Do not sign the body alone without the timestamp prefix.

Receivers must reject events when `|now - t| > 300` seconds (five-minute replay window).

Delivery uses up to three attempts with backoff 1s, 2s, 4s, five-second timeout, `redirect: "manual"`, and response bodies discarded (not stored in the log).

## Verify in Node

```javascript runnable
import { createHmac, timingSafeEqual } from 'node:crypto';
import http from 'node:http';

const SECRET = process.env.TAKEHOME_WHSEC;
const TOLERANCE_SEC = 300;

function verifySignature(secret, rawBody, header, nowSec) {
  const parts = Object.fromEntries(
    header.split(',').map((p) => p.trim().split('=')),
  );
  const t = Number.parseInt(parts.t, 10);
  const v1 = parts.v1;
  if (!Number.isFinite(t) || !v1) return false;
  if (Math.abs(nowSec - t) > TOLERANCE_SEC) return false;
  const expected = createHmac('sha256', secret)
    .update(`${t}.${rawBody}`)
    .digest('hex');
  const a = Buffer.from(v1, 'hex');
  const b = Buffer.from(expected, 'hex');
  if (a.length !== b.length) return false;
  return timingSafeEqual(a, b);
}

const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (c) => chunks.push(c));
  req.on('end', () => {
    const rawBody = Buffer.concat(chunks).toString('utf8');
    const header = req.headers['takehome-signature'] ?? '';
    const ok = verifySignature(SECRET, rawBody, header, Math.floor(Date.now() / 1000));
    res.statusCode = ok ? 200 : 401;
    res.end(ok ? 'ok' : 'invalid');
  });
});
server.listen(3456);
```

Run with `TAKEHOME_WHSEC=whsec_your_secret node verify-webhook.mjs`.

## Verify in Python

```python runnable
import hashlib
import hmac
import os
import time
from http.server import BaseHTTPRequestHandler, HTTPServer

SECRET = os.environ["TAKEHOME_WHSEC"].encode("utf-8")
TOLERANCE_SEC = 300


def verify_signature(secret: bytes, raw_body: bytes, header: str, now_sec: int) -> bool:
    parts = dict(p.split("=", 1) for p in header.split(",") if "=" in p)
    t_str = parts.get("t")
    v1 = parts.get("v1")
    if not t_str or not v1:
        return False
    t = int(t_str)
    if abs(now_sec - t) > TOLERANCE_SEC:
        return False
    msg = f"{t}.".encode("utf-8") + raw_body
    expected = hmac.new(secret, msg, hashlib.sha256).hexdigest()
    return hmac.compare_digest(v1, expected)


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw_body = self.rfile.read(length)
        header = self.headers.get("takehome-signature", "")
        ok = verify_signature(SECRET, raw_body, header, int(time.time()))
        self.send_response(200 if ok else 401)
        self.end_headers()
        self.wfile.write(b"ok" if ok else b"invalid")


if __name__ == "__main__":
    HTTPServer(("127.0.0.1", 3457), Handler).serve_forever()
```

Run with `TAKEHOME_WHSEC=whsec_your_secret python3 verify_webhook.py`.

## Delivery log and replay

`GET /v1/webhooks/deliveries` returns `deliveries[]` with `status`, `attempts`, `last_error`, `replay_of`, and timestamps. Response bodies from your server are never persisted.

Replay a failed delivery: `POST /v1/webhooks/deliveries/{id}/replay`. The new attempt links back through `replay_of`.

Test fan-out (your account only): `POST /v1/webhooks/dispatch` with `{ "from": "2026-01-01", "to": "2026-07-01" }`.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
