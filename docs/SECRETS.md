# Secrets index

This file lists every secret Takehome touches. It contains **no secret
values**. If a value appears here, that is a defect — rotate the secret and
fix the commit.

Operator: the person who can log into Cloudflare, Stripe, npm, and PyPI for
this product. Today that is the repository owner.

Rotation is always at the provider first. History rewriting is optional and
never a substitute for rotation.

---

## Provider credentials (not in git)

| Secret | What it is | Where it lives | Who rotates | If it leaks |
|--|--|--|--|--|
| Cloudflare API token | Wrangler / Pages / Worker deploy, D1 admin | Operator machine (`wrangler login` session or `CLOUDFLARE_API_TOKEN`). Never committed. Set via `wrangler secret` / dashboard, not `wrangler.toml`. | Operator, Cloudflare dashboard → API Tokens. Burn the token. | Attacker can deploy code, read D1 (accounts, hashed keys, webhook secrets), change DNS/routes on `takehome.gautamkhosla.com`. |
| Stripe secret key (`STRIPE_SECRET_KEY`) | Server-side Stripe API | Cloudflare Worker secret (`wrangler secret put STRIPE_SECRET_KEY`). Read in `services/api/src/stripe.js`. | Operator, Stripe dashboard → API keys. Roll the secret key. | Attacker can create Checkout sessions, read customers, and change subscriptions. |
| Stripe webhook signing secret (`STRIPE_WEBHOOK_SECRET`) | Verifies Stripe → Worker webhook signatures | Cloudflare Worker secret (`wrangler secret put STRIPE_WEBHOOK_SECRET`). Used in `services/api/src/handler.js`. | Operator, Stripe dashboard → webhook endpoint signing secret. | Attacker can forge `checkout.session.completed` and upgrade plans without paying. |
| Stripe price IDs (`STRIPE_PRICE_STARTER`, `STRIPE_PRICE_GROWTH`, `STRIPE_PRICE_BUSINESS`) | Live Stripe Price objects for CAD plans | Cloudflare Worker vars/secrets. Tests use non-secret placeholders `price_starter_cad` / `price_growth_cad` / `price_business_cad`. | Operator, Stripe dashboard. | Not a credential. A wrong ID bills the wrong product. Treat production IDs as production config, not git fixtures. |
| `NPM_TOKEN` | Publish `takehome-ca` to npm | Operator environment only. `scripts/publish-packages.sh` refuses to run without it. Must not be committed; `.npmrc` is gitignored. | Operator, npm → Access Tokens. Revoke the token. | Attacker can publish a malicious `takehome-ca` that payroll developers install. Supply-chain failure (OWASP A03:2025). |
| `PYPI_API_TOKEN` | Publish `takehome-ca` via `maturin publish` | Operator environment only. Same script. | Operator, PyPI → API tokens. Revoke the token. | Same as npm: a poisoned wheel on `pip install takehome-ca`. |
| GitHub Actions `GITHUB_TOKEN` | `rules-watch` opens issues on T4127 pin drift | Injected by GitHub Actions (`secrets.GITHUB_TOKEN`). Workflow requests `issues: write`. | GitHub (ephemeral per job). If a workflow is compromised, rotate by locking the workflow and reviewing Actions history. | Attacker with a writeable workflow can open issues; they cannot push unless a PAT with `contents: write` is added — do not add one. |

---

## Secrets issued to customers (not in git)

| Secret | What it is | Where it lives | Who rotates | If it leaks |
|--|--|--|--|--|
| `np_test_` / `np_live_` API keys | Bearer keys for `/v1/deductions*`. Live keys are metered. | Shown once at signup verify. Stored in D1 as SHA-256 (`services/api/src/keys.js`). Plaintext is not retained. | Customer: sign up again / we add a rotate endpoint later. Operator can delete the D1 row. | Attacker can run calculations as that account. Live keys can exhaust the plan; they cannot move money. |
| Customer webhook signing secrets (`whsec_…`) | HMAC key for `takehome-signature` on `rule_set.changed` deliveries | Generated at `POST` webhook register. Returned **once** in the create response. Stored **plaintext** in D1 `webhook_endpoints.secret`. | Customer deletes and re-registers the endpoint (new secret). Operator can `DELETE` the row. | Attacker can forge rule-change payloads to that customer’s webhook URL. Rotate by deleting the endpoint. |
| Signup verification tokens | 24-byte hex token in `/signup/verify/?token=` | Hashed in D1 `email_tokens`. 24-hour expiry. | Expires. Operator can delete the D1 row. | Attacker who has the URL can issue keys for that email. See *Signup mail* below. |

---

## Identifiers that are not credentials

These are in git on purpose. They do not grant access by themselves.

| Identifier | Where | Notes |
|--|--|--|
| D1 `database_id` / `database_name` | `services/api/wrangler.toml` (`[[d1_databases]]`) | Cloudflare UUID for database `takehome`. Binding is `DB`. Access still requires a Cloudflare token. If the token leaks, this ID tells an attacker which database to query — rotate the token, not the UUID. |
| Worker name / Pages project | `takehome-api`, Pages project `takehome` | Public origin `takehome.gautamkhosla.com`. |
| Stripe test placeholders | `services/api/tests/helpers.js`, `scripts/local.mjs` | `price_*_cad` strings. Not Stripe objects. |

---

## Signup mail (no SMTP credential exists)

`sendMail` in `services/api/src/handler.js` only pushes to `env.MAILBOX` (in-memory, tests and `npm run local`). There is **no** SendGrid / Postmark / SES / SMTP secret in this repo or in Worker secrets as of this index.

Production `wrangler.toml` sets `ECHO_VERIFY_URL = "1"`, so the verify URL (and therefore the signup token) is returned in the signup JSON. That is a control choice, not a stored credential: anyone who can `POST /v1/signup` for an email they do not control can still not read another person’s mailbox, but they **can** complete verify if they see the JSON (smoke tests rely on this). Before public launch, decide whether production keeps the echo (test-only) or a real mailer with a mail-provider API key added to this index.

---

## Operator-local, never git

| Item | Notes |
|--|--|
| `~/.npmrc` auth token | Operator npm login. Gitignored as `.npmrc`. A stale token that npmjs rejects with 401 is still a secret until revoked at npm. |
| `~/.pypirc` / `TWINE_PASSWORD` | Not present. When created, gitignore already covers typical pypirc if we add it; prefer `PYPI_API_TOKEN` in the shell, not a file in the repo. |
| Wrangler OAuth session | `~/.wrangler` / `.wrangler/` (gitignored). |
| `.env` / `.dev.vars` | Must never be committed. Gitignored. Worker secrets go through `wrangler secret put`. |

---

## Stage 0 scan (2026-09-19)

- **gitleaks** `v8.30.1` over `--all --full-history`: one finding. `docs/FIVE-MINUTE-TEST.md` (commit `978d611`) matched `curl-auth-header` on the documented placeholder `np_test_YOUR_KEY`. Same placeholder is in `README.md`. **Not a live key.** Triaged; `.gitleaks.toml` allowlists that exact string.
- **trufflehog** `v3.90.8` `git file://. --only-verified`: zero verified secrets, zero unverified secrets.
- Git history still contains the `978d611` blobs for `packages/takehome-py/python/takehome_ca/_native.abi3.so` and two `.pyc` files, removed from the tree in `482cef4`. Those are build artifacts, not credentials. They stay until someone chooses a history rewrite. `scripts/tracked-ban.sh` is the check so they cannot re-enter `HEAD`.
