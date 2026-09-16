# Conformance operations — CRA PDOC oracle

**Status:** Published before the first automated request from this repository’s
Playwright harness (`tools/pdoc-oracle/`).

**Audience:** Maintainers of Netpay, CRA/PDOC operators who may observe our
traffic, and anyone auditing how vector `expected` amounts are obtained.

**Related:** Root [`CONFORMANCE.md`](../CONFORMANCE.md) records measured
results and disagreements. This document is the *operating* policy for
contacting PDOC.

---

## 1. Purpose

Netpay implements CRA T4127 payroll deduction formulas. The authoritative
interactive check for period amounts is the CRA **Payroll Deductions Online
Calculator** (PDOC):

https://apps.cra-arc.gc.ca/ebci/rhpd/beta/

We drive that form only to:

1. Capture **frozen oracle vectors** for automated regression (M1 twenty cases,
   then the larger M2 grid).
2. Record **provenance** (timestamp, input, output, PDOC version string,
   browser version, screenshot hash) so every cent in `expected` is
   attributable to a PDOC result — never to our engine.

We are **not** scraping PDOC for a product feature, rate API, or live proxy.
The harness is a **standalone developer tool**. It is never part of a deployed
Netpay artifact and does not import `netpay-core`.

This policy exists so automated use is distinguishable from a scraper: honest
identity, published rate limits, permanent cache, and overnight batching.

---

## 2. Contact

| | |
|--|--|
| **Project** | Netpay (`netpay-ca/netpay`) |
| **Repository** | https://github.com/netpay-ca/netpay |
| **Conformance record** | https://github.com/netpay-ca/netpay/blob/main/CONFORMANCE.md |
| **This policy** | https://github.com/netpay-ca/netpay/blob/main/docs/CONFORMANCE-OPERATIONS.md |
| **Issues / abuse reports** | https://github.com/netpay-ca/netpay/issues |
| **Product site** | https://netpay.ca |

If CRA or Shared Services Canada needs us to stop, slow further, or change
identity strings, open an issue on the repository or contact the maintainers
via that tracker. We will treat such requests as stop conditions for the
harness until resolved.

---

## 3. Client identity (User-Agent)

Every automated browser session sets a User-Agent that **identifies the client
honestly** and includes a **contact URL**. We do not spoof a generic consumer
browser string alone.

Example shape (exact string may include a harness version):

```text
Netpay-PDOC-Oracle/0.1 (+https://github.com/netpay-ca/netpay; conformance; contact=https://github.com/netpay-ca/netpay/blob/main/docs/CONFORMANCE-OPERATIONS.md)
```

Human-operated captures (Cursor browser, manual transcription) are labelled
`operator: "manual"` in vector metadata and are not subject to this User-Agent,
but they still must not fill `expected` from the engine.

Automated runs use `operator: "pdoc-oracle-harness"`.

---

## 4. Rate, backoff, and stop conditions

Non-negotiable for every automated session:

| Rule | Value |
|------|--------|
| Minimum gap between **network** requests that hit PDOC | **3 seconds** |
| Non-HTTP-200 (or failed navigation / form submit) | **Exponential backoff** before retry |
| Consecutive hard failures | **Hard stop after 3**, loud error, no further requests that session |
| **Heartbeat** | **Every 30 seconds** to `data/pdoc-cache/queue-progress.log` (`heartbeat attempted=… idle_s=…`) |
| **Stall** | **No completed form for 5 minutes** → **non-zero exit**, checkpoint saved. A hung browser is not a rate limit. |
| Scheduling | **Overnight / scheduled batches only** — never a burst of back-to-back fetches |
| Cache hit | **Zero** network contact with PDOC |

“Request” means any navigation or form submission that loads a PDOC page.
Local cache reads do not count toward spacing.

Backoff sketch: after failure *n* (1-based), wait at least
`min(3 × 2^(n-1), 180)` seconds before one retry attempt, still respecting the
3-second floor between attempts. After three consecutive failures, exit
non-zero and leave remaining cases untouched.

We do **not** parallelize multiple browsers against PDOC. One session, one
queue.

---

## 5. Permanent cache (fetch once, ever)

Cache directory (default): `data/pdoc-cache/` at the repo root (gitignored
payloads; optional index may be committed separately if maintainers choose).

**Key:**

```text
sha256( canonical_input_json || rule_set_version )
```

- `canonical_input_json` is a stable serialization of the case input (sorted
  object keys, lexical money strings, no insignificant whitespace variance).
- `rule_set_version` is the Netpay / CRA effective rule-set id for the case
  (e.g. `2026-01-01`), concatenated as a string after the JSON.

**Invariants:**

1. A PDOC result for a given input and rule set is treated as **immutable**.
2. Each distinct key is fetched from the network **at most once, ever**.
3. On cache hit, the harness **must not** open PDOC or take a new screenshot.
4. Cache entries store the full measurement record (see §6), including the
   screenshot bytes or their hash and path.

If PDOC later changes behaviour for the same calendar rules, that is a
**new** disagreement investigation: we do not silently re-fetch to “update”
frozen vectors. Changing a published vector requires an explicit maintainer
decision and a CONFORMANCE.md note.

The immutability claim in (1) holds **only while PDOC itself is unchanged**.
See §10.2 for the identity alarm.

---

## 6. What every measurement records

For every successful capture (network or restored from cache when emitting
vectors), the record includes:

| Field | Meaning |
|-------|---------|
| Timestamp (`retrieved_at`) | ISO-8601 UTC when the result was obtained from PDOC (cache stores the original) |
| Full input | Exact request / form fields used |
| Full output | Amounts mapped into the step-12 vector `expected` shape |
| PDOC version string | Whatever version text appears on the page (e.g. footer `2026-06-11`) |
| Browser version | Playwright / Chromium (or other) version string |
| Screenshot hash | SHA-256 of the results-page screenshot |
| Operator | `pdoc-oracle-harness` for this tool |
| PDOC identity | Version string, or form-structure hash if no version is exposed (§10.2) |
| robots.txt hash | SHA-256 of each robots.txt body fetched for this run (§10.1) |
| `observed_edition` | Calendar rule-set edition live PDOC was **serving** at capture time (e.g. `2026-07-01` for the 123rd edition). Distinct from `rule_set_version`, which is the case key. |
| `rule_set_version` | Netpay / CRA effective rule-set id the case was generated against |

A cache record may satisfy a case only when `observed_edition` equals the
case `rule_set_version`, **or** `data/edition-identity.json` proves every
jurisdiction the case touches is field-identical between those editions
(test 18 jurisdiction tables + test 19 claim-code files). Anything else is
a hard error — including January-only BC/NL/PE forms against a July-serving
PDOC, which are **edition-retired**, not pending.

Emitted vector records use **exactly** the step-12 format consumed by
`crates/netpay-core/tests/pdoc_vectors.rs` (`id`, `description`, `request`,
`expected`, `oracle`, optional `notes`).

**Never** write engine output into `expected`. Only PDOC (or a documented
manual transcription of a PDOC screen) may fill those fields.

---

## 7. Batch plan

1. **M1 first:** the twenty Ontario cases in
   `crates/netpay-core/tests/vectors/pdoc_ontario_2026_01.json`.
   Prefer cache hits for already-filled manual captures where the harness can
   import or re-key them; otherwise fetch only `PENDING_PDOC` rows, at policy
   rate.
2. **Then M2 grid:** continue overnight with the larger case list, same
   spacing and hard-stop rules, writing new cache entries and vector files
   without touching the engine.

Operators should start batches outside peak daytime hours for the service
region when practical (“scheduled overnight”). Daytime ad-hoc runs of a few
cache-miss cases are allowed for debugging but must still obey §4 — never
compress the queue into a burst.

---

## 8. Reconciliation discipline

Disagreements of one to several cents are expected at the boundaries (mid-year
K2, annualized vs YTD contribution bases, transcription). They are resolved by:

1. Leaving PDOC `expected` unchanged.
2. Documenting input, both outputs, delta, and explanation in
   [`CONFORMANCE.md`](../CONFORMANCE.md).
3. Fixing the **engine** (or documenting a deliberate wire-contract difference),
   not by tuning until vectors pass.

Papering over M1 disagreements destroys M2 credibility.

---

## 9. Change control

Amendments to rate limits, identity, or cache semantics are made in this file
**before** the harness behaviour changes. The harness must refuse to run if
this document is missing from the checkout (fail closed).

---

## 10. Pre-flight: robots.txt and PDOC identity

Both checks run **before** the case queue. A failure is a hard stop: no
cache-miss fetches, no form submits, no “just one more.”

### 10.1 Honour robots.txt

The harness **fetches, parses, and obeys** robots.txt for every origin it
will contact, including `https://www.canada.ca/robots.txt` and the PDOC
origin `https://apps.cra-arc.gc.ca/robots.txt`.

| Rule | Behaviour |
|------|-----------|
| Fetch | Honest User-Agent (§3). Record HTTP status, final URL, and **SHA-256 of the response body**. |
| Parse | RFC 9309 groups (`User-agent`, `Allow`, `Disallow`). Longest matching path rule wins. Our product token is `Netpay-PDOC-Oracle`; otherwise `*`. |
| 404 / missing file | No robots.txt → **no restrictions** for that origin (RFC 9309). Still record status + body hash so a later 200 cannot be confused with “we never checked.” |
| 5xx / network failure | **Fail closed.** Do not treat an outage as permission. |
| Disallow of a PDOC path | **Stop.** Do not discover a ban after thousands of requests. We find another approach (manual capture, CRA-published tables) — we do not crawl around the disallow. |

robots.txt is origin-scoped. A rule on `canada.ca` does not legally govern
`apps.cra-arc.gc.ca`, but we still refuse if **either** origin’s file
disallows a PDOC path we intend to use on that origin, or if `canada.ca`
disallows a known PDOC URL on canada.ca itself.

Each run writes the fetched copies’ hashes into the run-conditions record
stored with the cache (`robots.txt` URL → sha256). Changing robots.txt
does not invalidate existing oracle vectors; it can only stop **new**
network contact.

**Check performed 2026-09-12 (before `src/pdoc.ts` existed):**

| Origin | Result | SHA-256 of body |
|--------|--------|-----------------|
| `https://www.canada.ca/robots.txt` | 200. `User-agent: *`. No rule matches PDOC / `/ebci/rhpd`. | `5e6c293b04d4b15808595bf89bb179003f0e74ead9ca5219ec25944a25d78826` |
| `https://apps.cra-arc.gc.ca/robots.txt` | **404** (HTML error page, not a robots file). Treated as no robots.txt. | `8be027a5a26315b017388187e1db558944b3d6304a8cb6456bca440f21bdf73a` |
| `https://www.cra-arc.gc.ca/robots.txt` | 200. `User-agent: *` / empty `Disallow:` (allow all). | `3cd9207e4a7ed35a9085b2facec36d08dd945ae9455b9ddd609b36fee9d57d2c` |

PDOC path `/ebci/rhpd/beta/` is **not disallowed**. The live harness repeats
this fetch at the start of every session; the table above is the pre-write
gate, not a substitute for the run-time check.

### 10.2 Alarm on PDOC version drift

Cache key `sha256(canonical_input_json || rule_set_version)` is correct: a
PDOC result for a given input and CRA rule set does not change **while
PDOC itself is unchanged**. That assumption is an operational invariant,
not a theorem.

**Identity.** Every cache record and every run stores a PDOC identity:

1. The version string shown on the page when one is exposed (today:
   footer-style dates such as `2026-06-11`).
2. If none is exposed, `sha256` of a canonical fingerprint of the **entry
   form’s structure** (control names, types, and labels — not values).

**Probe.** Once per automated session, after robots.txt and before the
case queue, the harness loads the PDOC entry page **once** to observe the
current identity. That single navigation counts as a PDOC request under
§4. Cache hits for individual cases still make **zero** further contact.

**Alarm.** Compare the observed identity to
`data/pdoc-cache/pdoc-identity.json` (the identity the cache was built
under).

- **Match:** proceed. Cache hits are valid for this run.
- **Mismatch:** **exit non-zero immediately.** Do not serve the cache as
  authoritative, do not start a re-fetch campaign, do not overwrite
  vectors. The message must say that the **entire cache may need
  re-validation** because PDOC changed. Re-validation is a maintainer
  decision recorded in [`CONFORMANCE.md`](../CONFORMANCE.md).
- **No identity file yet (empty cache):** write it after the first
  successful capture; do not invent one from the engine.

A silent identity change would otherwise look like a mysterious
conformance regression weeks later. This alarm is that signal, on
purpose.

## 11. Known capture hazards (before M2 grid)

### 11.1 Fixed TD1 dollars vs claim codes — BPAF phaseout

PDOC’s **Basic personal amount** can be entered two ways:

1. **Claim codes** (federal / provincial claim code 1, …) — the calculator
   applies the dynamic BPA / BPAF formula, including **phaseout** between the
   published start and end thresholds.
2. **Fixed TD1 dollars** (e.g. typing `16452.00` for the 2026 federal maximum)
   — that amount is treated as a **locked claim**. It does **not** phase out.

For high-income vectors (BPAF phaseout dimension, anything with annual income
at or above the phaseout start, and any grid cell that is meant to exercise
claim-code behaviour), capture with **Claim codes**, not a fixed dollar field
pre-filled with the published maximum. Using fixed `$16,452` on a phaseout
case silently corrupts that entire grid dimension: every cell looks “captured”
but the oracle never applied BPAF reduction.

This bit one M1 vector before correction. Write it into every M2 capture runbook.
Do not generate ~250,000 PDOC visits; see §13.

### 11.2 Oracle source: `/SALARY/calculate` JSON vs rendered DOM

The harness reads period amounts from PDOC’s **`/SALARY/calculate` JSON**
response (XSSI-prefixed), not from the results DOM. Reasons:

1. The Angular results page can crash mid-render after a successful calculate.
2. The JSON payload is stable and complete for the fields we map.

That endpoint is a **different interface** than a human user sees. It is
undocumented as a public API. We treat it as the capture oracle only because
it is what the salary form itself posts to.

**Spot-check policy:** on every new harness version, and at the start of each
overnight queue session, compare JSON figures to the rendered results for a
handful of cases (at least one exact match and one known M-003 midpoint-down
case when available). Record the check in the run checkpoint notes.
If JSON and DOM diverge on a mapped field, **hard stop** — do not trust the
queue until the mapping is reconciled.

**Spot-check performed 2026-09-12:** three cases (ON weekly claim1, AB P=10
M-003, BC biweekly claim0). Federal and provincial tax lines matched JSON ↔
rendered text. Full results DOM still often incomplete (CPP/EI/net lines
missing — the Angular hazard that motivated JSON capture). `npm run
spot-check-json-dom` in `tools/pdoc-oracle`.

## 12. Grid version on every run

The M2 case list comes from `tools/grid-gen`. It is deterministic: same
`grid_version` (`2026.1`) and seed (`t4127-grid-2026.1`) produce byte-identical
output. Every conformance run — cache hit or capture — **records that
`grid_version`** in the run header and in [`CONFORMANCE.md`](../CONFORMANCE.md).
A number without a grid version is not a conformance number.

## 13. Sampling design (PDOC vs interior vs uncapturable)

The grid is 842,304 cells. PDOC is one session at 3 seconds per uncached
form. The published queue is every **distinct** July boundary form against
the **ten** pay periods the live salary UI exposes (not all fourteen legal
P). Forms with P ∈ {1, 2, 4, 2000}, `$0` gross, or sub-dollar gross are the
named **`uncapturable`** class: invariants + differential oracle, same as
the interior log-ladder. Root [`CONFORMANCE.md`](../CONFORMANCE.md) states
the split and the distinct-form count after fingerprint dedup. Do not queue
a run that cannot finish, and do not publish a shorter queue under the old
all-14-P headline.

---

*Last updated: 2026-09-12 — §11.2 JSON oracle; §13 ten-P queue + uncapturable; §12 grid version; §10 identity alarm.*
