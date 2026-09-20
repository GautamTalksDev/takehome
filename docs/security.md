# Security architecture

**Who this is for:** Operators, security reviewers, and integrators who need boundaries before they trust the hosted API with account data.

**When you finish:** You know what we store, what we never store, and where to report issues.

Threat modeling for the hosted product lives here. Engine correctness and PDOC conformance are separate topics ([`docs/conformance.md`](conformance.md), [`CONFORMANCE.md`](../CONFORMANCE.md)).

## Assets and scope

- **Customer email (PII):** Stored in `accounts.email` in D1 after signup verify.
- **Payment method (billing):** Deferred (ADR-006). Checkout returns `billing_unavailable`. No Stripe secrets in production until paid billing turns on.

- **API keys (`np_live_` / `np_test_`):** Credential. SHA-256 digest only in D1 ([ADR-004](ADR-004-api-key-hash.md)).
- **Webhook signing secrets:** Credential. Plaintext in D1; shown once at register.
- **Payroll calculation inputs:** Ephemeral. **Not persisted** (see below).
- **Rule JSON vendored in the engine:** Public Apache-2.0; same bits in WASM and npm.

**Out of scope for this document:** CRA PDOC availability, customer payroll systems downstream of our JSON, and physical access to operator laptops. Quebec (QPIP (Québec Parental Insurance Plan) and QPP) support is unsupported by design ([`docs/jurisdictions.md`](jurisdictions.md)).

## Structural: we never store salary

`/v1/deductions` accepts `gross_pay`, bonuses, and year-to-date fields in the request body. The Worker runs `takehome-core`, returns JSON, and increments usage metering. Migrations define `accounts`, `api_keys`, `usage`, `webhooks`, and Stripe linkage only. No column stores gross pay, net pay, or province for a calculation.

Browser calculators load WASM and compute locally after first fetch; gross pay does not leave the browser on that path ([diagram 06](diagrams/_include.md)).

## Threat model (hosted API)

- **Stolen API key:** Abuse of metered calculations. Mitigation: high-entropy keys; hash-at-rest; revoke via D1 row delete.
- **Forged Stripe webhook:** Endpoint returns `404` while billing is deferred. When billing returns: `STRIPE_WEBHOOK_SECRET` signature verify.
- **SSRF via customer webhook URL:** Worker probes internal networks. Mitigation: URL validation at register and immediately before fetch; no redirects ([diagram 07](../web/site/public/diagrams/07-webhook-ssrf-guard.svg)).
- **D1 outage during live call:** Unmetered calculations. Mitigation: fail closed; no `employee` payload ([ADR-005](ADR-005-metering-fail-closed.md)).
- **Signup token leak:** Key issuance for victim email. Mitigation: hashed token, 24h expiry; production must not set `ECHO_VERIFY_URL` ([`SECRETS.md`](SECRETS.md)).
- **Error response leak:** Path or SQL exposure. Mitigation: OWASP-oriented tests; typed errors, no stack traces (`services/api/tests/50-owasp-config.test.js`).

## Controls and boundaries

- **Authentication:** Bearer API keys on `/v1/deductions*`, keys, webhooks, signup (where applicable).
- **CORS:** Public read-only rule and conformance endpoints open; keyed routes closed (`50-owasp-config.test.js`).
- **Secrets:** Provider credentials only in Cloudflare Worker secrets and operator machines ([`SECRETS.md`](SECRETS.md)).
- **Engine isolation:** Rust core forbids IO, network, clock, and float arithmetic in library code (`.cursor/rules/takehome.mdc`).
- **Dependency and publish:** OIDC publish to npm/PyPI; no long-lived publish tokens in git.

There is no separate OWASP mapping markdown file in this repository. Control intent is encoded in API tests (files `50-owasp-config.test.js`, `54-insecure-design.test.js`, and related suites under `services/api/tests/`).

## Reporting

See root [`SECURITY.md`](../SECURITY.md) for coordinated disclosure and response commitments.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
