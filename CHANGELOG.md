# Changelog

> **2026-09-17:** Product renamed from Netpay to Takehome (GitHub `takehome-ca/takehome`, crates `takehome-core` / `takehome-wasm` / `takehome-cli`, npm/PyPI `takehome-ca`, binary `takehome`, site `takehome.gautamkhosla.com`). Entries below keep the names they were written under.

## [Unreleased]

- Lighthouse gate restored after a 0.8 CI floor briefly landed (gate moved, not the regression fixed). Local/pre-push (`scripts/check-lighthouse-local.sh`, `TAKEHOME_LIGHTHOUSE_MODE=local`) enforces **performance ≥ 0.99** and **accessibility ≥ 0.99**. Shared GHA runners take **three** samples and assert **median(performance) ≥ 0.95**; accessibility stays **≥ 0.99 on every sample**. Evidence for the split: local recorded **1.00 / 1.00**; a cold GHA single run scored **performance 0.84** on `/calculators/on/` (WASM calculator page under runner load). Median-of-three against 0.95 absorbs that variance without turning the check into a no-op.
- Live smoke script covers batch, year CPP cap `4230.45`, CRA bonus combined `TB = 503.72` (federal-only `T3` slice remains `323.54`), BC Option 1/Option 2 at `2026-08-01`, and the 2027 preview banner above the number.
- Paid billing deferred for free early access (ADR-006): checkout is `501` / `billing_unavailable`, Stripe webhook is `404`, free live quota is **100,000**/month. Kill-test route C withdrawn.
- PDOC evidence publication (ruling a): JSON records are the GitHub release archive `takehome-conformance-corpus-2026.1.tar.zst` (not in git; `tracked-ban.sh` refuses `records/`). `CONFORMANCE.md` records the catalog digest `554a5427…d02a71ea`, the archive SHA-256, the download URL, and two lines to verify after download. Publication tests extract the archive and break the catalog on purpose.
- Documentation house style: `scripts/docs-lint.sh` fails on em dashes, banned words, missing audience blocks, and undefined domain terms. Seven Mermaid diagrams render to SVG at site build. Docs nav is grouped (Getting started, Concepts, API reference, Conformance and findings, Operations). Link check and runnable example harness are in CI.
- Gzipped WASM went **141,104 → 186,011** because `wasm-opt` was gated behind `WASM_OPT=1` to freeze the hash. Pinning Binaryen **132** was the right determinism fix (`check-binaryen-version.sh` fails on drift). Separate finding: `-Oz` shrinks uncompressed 614 KB → 551 KB but gzip goes slightly **up**, and **141,104 was the smaller 0.4.0 module**, not an optimised M4 build. The M4 growth is real code size. Gzipped size is **199,744** (CI max 220,000): about **9% headroom**, tighter than before.

## [0.5.0-m4] - 2026-09-18
- `POST /v1/deductions/batch`: up to 1000 calculations, results in input order, partial success (`{ok, response}` or `{error}`), 1001 rejected by name, metered as N not 1, over-quota is 402 with usage unchanged, byte-identical replay.
- `POST /v1/deductions/year`: every pay period in the year with YTD CPP/CPP2/EI carried forward. CPP and EI caps, YMPE crossing, CPP2 start (spec §21.5), BC mid-year rule-set split, 53-week / 27-biweekly exemptions. Federal tax sum vs T1 differs by at most one cent per period because of per-period rounding.
- T4127 Chapter 5 Option 2 cumulative averaging (§6.12): S1 is an exact pair (`52/1`, `52/2`), A and T use S1 / M / M1, K2 uses the Chapter 5 form, and BC/NL/PE mid-year proration is Option 1 only. PDOC does not expose Option 2; evidence is T4127 worked examples plus invariants, a weaker oracle class that does not inherit the Option 1 PDOC agreement rate.
- Rule-change detection (§12.3): `rules-watch` pins the T4127 index, edition pages, and CSV bundle; a hash mismatch opens a GitHub issue with the unified diff. `GET /v1/rules/diff` compares two editions field by field. Account webhooks deliver an HMAC-signed `rule_set.changed` payload with retries, a delivery log, and replay.
- 2027-01-01 PREVIEW rule set (proposed, not enacted): CPP employee total 5.75% (base 4.75%), maximums recalculated against held 2026 YMPE/YAMPE, BC indexation paused at 2026 levels. A 2027 `as_of` returns the set with a `RULE_SET_PROPOSED` warning naming the 2026-04-28 Spring Economic Update. `/cpp-2027-rate-change` computes under it with the uncertainty above the number.
- Conformance re-run against the M4 engine: overall PDOC rate still `8838/9002`. New classes named: `pdoc-bonus-2026` (20 PENDING_PDOC, excluded from the rate), `year-projection-2026` (invariants), `t4127-option2-2026` (T4127 worked examples; PDOC caveat). All 164 M-003 disagreements remain listed. Integer-cent probe of `rounding_compat: pdoc` found no discriminator; the typed error stays.
- Five-minute classmate run is still an empty row in `docs/FIVE-MINUTE-TEST.md`. `takehome-ca` is not on npm or PyPI yet.

## [0.4.0-m3] - 2026-09-17
- Product renamed from Netpay to Takehome (host `takehome.gautamkhosla.com`).
- Design day: wordmark, `:root` tokens including dark, navbar, result panel, citations, copy, motion, five equal pricing cards, docs with shiki and copy. Lighthouse performance 1.00 and accessibility 1.00; JS excluding WASM under the 50KB gate.
- Embeddable widget: `<script src="https://takehome.gautamkhosla.com/embed.js" data-province="ON" data-gross="75000"></script>` runs the WASM engine in a shadow root (no style leak either direction). Classic script ≤4096 bytes. Fixture matches the engine. Demo at `/embed/`.
- Origin live: Cloudflare Pages project `takehome`, Worker `takehome-api` on `takehome.gautamkhosla.com` (API `/v1*`, Pages build for the rest). Live smoke: one calculator page `800.79`, one authenticated `/v1/deductions` `800.79`, `/conformance` lists all 164 disagreements.
- Five-minute classmate run is still an empty row in `docs/FIVE-MINUTE-TEST.md`. It cannot be simulated.
- `takehome-ca` is not on npm or PyPI yet: no publish tokens in this environment.
- WASM (`netpay-wasm`) and npm `netpay-ca`: same JSON `calculate` as native; identity tests cover the twenty M1 vectors and 500 grid cases. Gzipped WASM is 141,104 bytes (CI max 160,000).
- Python `netpay-ca` on the same wire.
- Static calculator template (`web/site`): WASM in the browser, offline after first load, no salary leaves the browser. Playwright proves engine match, offline recalc, and zero network during a calculation.
- Two hundred HTML pages from four templates (employee, employer, situational, reference). `/conformance` renders the full disagreement list. `/changes` plus RSS. `/cpp-2027-rate-change` published before the 2027 T4127 edition exists. Test 10: render, compute, sitemap, internal link graph.
- Cloudflare Worker API (`services/api`): POST `/v1/deductions` and the M3 GET surfaces over the WASM engine. Unknown fields are named; out-of-range `as_of` is not nearest-matched; every response carries engine identity; OpenAPI is generated from `src/schema.js`.
- Self-serve keys and billing: `np_test_` / `np_live_` Bearer keys stored hashed; metering counts calculations not HTTP requests; usage headers; hard stop at the plan limit; Stripe Checkout in CAD; signup is email → verify → keys → first call. No contact-sales path.
- Five-minute developer path: `/docs/` from the header, signup, authenticated first call `net_pay` `800.79`. Script and timed runs in `docs/FIVE-MINUTE-TEST.md`.

## [0.3.0-m2] - 2026-09-15
- Thirteen-jurisdiction T4127 engine (both 2026 editions); Quebec remains typed unsupported.
- July PDOC queue finished: 8,818/8,982 compared forms; 164 one-cent M-003 disagreements counted; 780 step-2 non-advances named out of the rate (8,982 + 780 = 9,762).
- Overall PDOC agreement `8,838/9,002` including M1. Finding 002 remains open; `rounding_compat: pdoc` stays NotImplemented.

## [0.2.0-m1] - 2026-09-12
- M0: fixed-point decimal type and CRA rounding rules.
- M0: scaffold tree (wasm/cli placeholders, data/tools/web layout), kill test, float ban.
- M1: Ontario PDOC 20/20.
