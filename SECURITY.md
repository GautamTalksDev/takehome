# Security policy

**Who this is for:** Anyone who discovers a vulnerability in Takehome-hosted services, the public API, or published packages.

**When you finish:** You know how to report safely, what is in scope, and what response to expect.

## Reporting a vulnerability

**Preferred:** [GitHub private vulnerability reporting](https://github.com/takehome-ca/takehome/security/advisories/new) on `takehome-ca/takehome`.

**Alternate:** Email `security@takehome.ca` with the same details if GitHub is unavailable. Use encrypted mail if your report includes exploit code or customer data.

Please include reproduction steps, impact, and affected URLs or package versions. Do not open a public issue for exploitable problems.

## Scope

**In scope**

- `takehome.gautamkhosla.com` API and site (Cloudflare Worker + Pages)
- Authentication, metering, signup, webhooks, and Stripe integration in `services/api`
- Published npm (`takehome-ca`) and PyPI (`takehome-ca`) packages built from this repository
- WASM and native binaries shipped from tagged releases

**Out of scope**

- CRA PDOC itself (report to CRA)
- Customer misconfiguration of webhook URLs pointing at third-party hosts
- Denial of service against PDOC during conformance capture (see [`docs/CONFORMANCE-OPERATIONS.md`](docs/CONFORMANCE-OPERATIONS.md))
- Issues in dependencies without a Takehome-specific impact (report upstream; tell us if a bundled release needs a bump)

Calculation disagreements with PDOC that reflect documented findings (M-002, M-003) are conformance records, not security defects.

## Response commitment

We aim to acknowledge reports within **three business days**. We will confirm scope, ask for missing details once, and ship a fix or mitigation for confirmed issues on the default branch before public disclosure when possible.

We credit reporters in the advisory when they want attribution. We do not pursue legal action against good-faith research that follows this policy.

## Safe harbor

Research must avoid privacy violations, data destruction, and service disruption beyond what is needed to demonstrate impact. Do not access other users’ accounts or calculation data (we do not store salary inputs; do not exfiltrate emails or keys from D1).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
