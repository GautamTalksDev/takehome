# PDOC oracle harness

Drives the CRA [Payroll Deductions Online Calculator](https://apps.cra-arc.gc.ca/ebci/rhpd/beta/entry)
under [`docs/CONFORMANCE-OPERATIONS.md`](../../docs/CONFORMANCE-OPERATIONS.md).

**Never** write engine output into `expected`.

## Pre-flight (2026-09-12, before `src/pdoc.ts`)

`www.canada.ca/robots.txt` does not disallow PDOC.
`apps.cra-arc.gc.ca/robots.txt` is 404 (no robots.txt → no restrictions).
The harness re-fetches both at the start of every session (§10.1).

## Commands

```bash
cd tools/pdoc-oracle
npm ci
npm test
npm run check-robots   # robots.txt only — no PDOC
# npm run probe        # one PDOC entry-page identity load; do this on purpose
```

`capture` is gated until Appendix P field locators land. Do not guess form ids.

## Cache

`data/pdoc-cache/` (gitignored payloads). Key:
`sha256(canonical_input_json || rule_set_version)`.

A new run that observes a different PDOC version (or form-structure hash)
than `pdoc-identity.json` **exits non-zero** and does not drain the queue
(§10.2).
