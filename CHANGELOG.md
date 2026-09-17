# Changelog

> **2026-09-17:** Product renamed from Netpay to Takehome (GitHub `takehome-ca/takehome`, crates `takehome-core` / `takehome-wasm` / `takehome-cli`, npm/PyPI `takehome-ca`, binary `takehome`, site `takehome.gautamkhosla.com`). Entries below keep the names they were written under.

## [Unreleased]
- WASM (`netpay-wasm`) and npm `netpay-ca`: same JSON `calculate` as native; identity tests cover the twenty M1 vectors and 500 grid cases. Gzipped WASM is 141,104 bytes (CI max 160,000).
- Python `netpay-ca` on the same wire.
- Static calculator template (`web/site`): WASM in the browser, offline after first load, no salary leaves the browser. Playwright proves engine match, offline recalc, and zero network during a calculation.
- Two hundred HTML pages from four templates (employee, employer, situational, reference). `/conformance` renders the full disagreement list. `/changes` plus RSS. `/cpp-2027-rate-change` published before the 2027 T4127 edition exists. Test 10: render, compute, sitemap, internal link graph.
- Cloudflare Worker API (`services/api`): POST `/v1/deductions` and the M3 GET surfaces over the WASM engine. Unknown fields are named; out-of-range `as_of` is not nearest-matched; every response carries engine identity; OpenAPI is generated from `src/schema.js`.
- Self-serve keys and billing: `np_test_` / `np_live_` Bearer keys stored hashed; metering counts calculations not HTTP requests; usage headers; hard stop at the plan limit; Stripe Checkout in CAD; signup is email → verify → keys → first call. No contact-sales path.
- Five-minute developer path: `/docs/` from the header, signup, authenticated first call `net_pay` `800.79`. Script and timed runs in `docs/FIVE-MINUTE-TEST.md`.
- Embeddable widget: `<script src="https://netpay.ca/embed.js" data-province="ON" data-gross="75000"></script>` runs the WASM engine in the host page. Demo and pitch at `/embed/`.

## [0.3.0-m2] - 2026-09-15
- Thirteen-jurisdiction T4127 engine (both 2026 editions); Quebec remains typed unsupported.
- July PDOC queue finished: 8,818/8,982 compared forms; 164 one-cent M-003 disagreements counted; 780 step-2 non-advances named out of the rate (8,982 + 780 = 9,762).
- Overall PDOC agreement `8,838/9,002` including M1. Finding 002 remains open; `rounding_compat: pdoc` stays NotImplemented.

## [0.2.0-m1] - 2026-09-12
- M0: fixed-point decimal type and CRA rounding rules.
- M0: scaffold tree (wasm/cli placeholders, data/tools/web layout), kill test, float ban.
- M1: Ontario PDOC 20/20.
