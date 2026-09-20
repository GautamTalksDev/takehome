# Secrets index

This file lists every secret Takehome touches. It contains **no secret
values**. If a value appears here, that is a defect: rotate the secret and
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
| GitHub Actions `GITHUB_TOKEN` | `rules-watch` opens issues on T4127 pin drift; `publish.yml` attaches SBOMs to the GitHub Release | Injected by GitHub Actions (`secrets.GITHUB_TOKEN`). Job-level `permissions:` only: `issues: write` on rules-watch, `contents: write` + `id-token: write` on npm publish. | GitHub (ephemeral per job). If a workflow is compromised, lock the workflow and review Actions history. | Attacker with a writeable workflow can open issues or attach release assets; they cannot push unless a PAT with `contents: write` is added: do not add one. |

`takehome-ca` does **not** use `NPM_TOKEN` or `PYPI_API_TOKEN`. Publish is GitHub Actions OIDC: npm Trusted Publisher (`npm publish --provenance`) and PyPI Trusted Publishing (`pypa/gh-action-pypi-publish`). Those are provider-side bindings to `.github/workflows/publish.yml`, not secrets in this repo. Configure them on npmjs.com and pypi.org before the first Release. `scripts/publish-packages.sh` exits 1 so a laptop cannot ship a tarball.

---

## Secrets issued to customers (not in git)

| Secret | What it is | Where it lives | Who rotates | If it leaks |
|--|--|--|--|--|
| `np_test_` / `np_live_` API keys | Bearer keys for `/v1/deductions*`. Live keys are metered. | Shown once at signup verify. Stored in D1 as SHA-256 (`services/api/src/keys.js`). Plaintext is not retained. | Customer: sign up again / we add a rotate endpoint later. Operator can delete the D1 row. | Attacker can run calculations as that account. Live keys can exhaust the plan; they cannot move money. |
| Customer webhook signing secrets (`whsec_…`) | 256-bit HMAC key for `takehome-signature` on `rule_set.changed` deliveries | Generated at `POST` webhook register. Returned **once** in the create response (`GET` never includes it). Stored **plaintext** in D1 `webhook_endpoints.secret`. | Customer deletes and re-registers the endpoint (new secret). Operator can `DELETE` the row. | Attacker can forge rule-change payloads to that customer’s webhook URL. Rotate by deleting the endpoint. |
| Signup verification tokens | 24-byte hex token in `/signup/verify/?token=` | Hashed in D1 `email_tokens`. 24-hour expiry. | Expires. Operator can delete the D1 row. | Attacker who has the URL can issue keys for that email. See *Signup mail* below. |

---

## Identifiers that are not credentials

These are in git on purpose. They do not grant access by themselves.

| Identifier | Where | Notes |
|--|--|--|
| D1 `database_id` / `database_name` | `services/api/wrangler.toml` (`[[d1_databases]]`) | Cloudflare UUID for database `takehome`. Binding is `DB`. Access still requires a Cloudflare token. If the token leaks, this ID tells an attacker which database to query: rotate the token, not the UUID. |
| Worker name / Pages project | `takehome-api`, Pages project `takehome` | Public origin `takehome.gautamkhosla.com`. |
| Stripe test placeholders | `services/api/tests/helpers.js`, `scripts/local.mjs` | `price_*_cad` strings. Not Stripe objects. |

---

## Signup mail (Resend)

Production and staging send verification and alert mail through Resend.

| Item | Where | Notes |
|--|--|--|
| `RESEND_API_KEY` | Worker secret (`wrangler secret put RESEND_API_KEY`) | Never in git or `wrangler.toml`. Required when `MAILBOX` is absent. |
| `MAIL_FROM` | `wrangler.toml` `[vars]` / `[env.staging.vars]` | `Takehome <noreply@gautamkhosla.com>`. Domain must be verified in Resend (SPF/DKIM). |
| `MAIL_REPLY_TO` | `wrangler.toml` `[vars]` | Human-readable reply address on verification mail (not a noreply). |
| `ALERT_EMAIL` | Worker secret | Operator inbox for quota 402 and other A09 alerts. Same Resend path as signup. |

`sendMail` in `services/api/src/mail.js`: if `env.MAILBOX` is set (tests / `npm run local`), push in-memory and return. Else POST to `https://api.resend.com/emails`. If neither MAILBOX nor `RESEND_API_KEY` is set, signup returns **503** `mail` (fail closed). Alert mail failures are logged as `alert_mail_failed` and do not change the HTTP response.

`ECHO_VERIFY_URL` is **not** a production var. It is set only in gitignored `services/api/.dev.vars` and in the test harness (`ECHO_VERIFY_URL: '1'`). The Worker treats any value other than the exact string `"1"` as off, including absent, empty, `"0"`, `"false"`, and `"true"`. A tracked `[vars]` assignment would return the verification URL (and therefore the signup token) to anyone who can `POST /v1/signup`.

---

## Operator-local, never git

| Item | Notes |
|--|--|
| `~/.npmrc` auth token | Operator npm login for installs. Gitignored as `.npmrc`. **Do not** put a publish token here. `takehome-ca` publishes from `.github/workflows/publish.yml` via OIDC (`npm publish --provenance`). Configure an npm Trusted Publisher for that workflow; revoke any leftover classic `NPM_TOKEN`. |
| `~/.pypirc` / `TWINE_PASSWORD` / `PYPI_API_TOKEN` | Not used. PyPI Trusted Publishing on `publish.yml` (`pypa/gh-action-pypi-publish` + `id-token: write`). If an old API token exists at pypi.org, revoke it. |
| Wrangler OAuth session | `~/.wrangler` / `.wrangler/` (gitignored). |
| `.env` / `.dev.vars` | Must never be committed. Gitignored. Worker secrets go through `wrangler secret put`. |

---

## Stage 0 scan (2026-09-19)

- **gitleaks** `v8.30.1` over `--all --full-history`: one finding. `docs/FIVE-MINUTE-TEST.md` (commit `978d611`) matched `curl-auth-header` on the documented placeholder `np_test_YOUR_KEY`. Same placeholder is in `README.md`. **Not a live key.** Triaged; `.gitleaks.toml` allowlists that exact string.
- **trufflehog** `v3.90.8` `git file://. --only-verified`: zero verified secrets, zero unverified secrets.
- Git history still contains the `978d611` blobs for `packages/takehome-py/python/takehome_ca/_native.abi3.so` and two `.pyc` files, removed from the tree in `482cef4`. Those are build artifacts, not credentials. They stay until someone chooses a history rewrite. `scripts/tracked-ban.sh` is the check so they cannot re-enter `HEAD`.

---

## Stage 5 scan (2026-09-20)

- **gitleaks** `v8.24.3` with `--log-opts=--all` (full history): **no leaks found**. Allowlisted placeholder still in `.gitleaks.toml`.
- **trufflehog** `v3.90.8` / `3.97.5` runtime `git file://. --only-verified`: **0 verified, 0 unverified**.
- `LICENSE-APACHE` (and package LICENSE files) replaced with the full Apache-2.0 text.
- `services/api` stays in the public tree; README licence split states the hosted product is metered, Worker source remains Apache-2.0.
- Conformance corpus archive rebuilt: `takehome-conformance-corpus-2026.1.tar.zst` SHA-256 `00a68d0bef8835f8271039981c22bcaca54ff5f0786c33fd3c5b346b27a0023f` (catalog digest `554a54278727bb460e7f2710cde2b6e31b750ea57c4a4ee6735eb2e4d02a71ea`). Tarball is not in git.
