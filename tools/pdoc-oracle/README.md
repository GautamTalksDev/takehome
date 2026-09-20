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
npm run backfill-m1    # import M1 twenty with observedEdition
npm run capture        # stratified 200-form smoke
npm run capture:bonus  # twenty pdoc-bonus-2026 vectors; never fills expected from the engine
# npm run capture:queue  # full capturable July queue — only after smoke + CONFORMANCE republish
```


Appendix P locators: `src/appendix-p.ts` (label/`name`-based; Angular ids are
ephemeral UUIDs). Result amounts come from `/SALARY/calculate` JSON because the
results DOM can crash mid-render — see ops §11.2 (JSON is not the human UI;
spot-check JSON vs rendered on a handful of cases per session).

## Cache

`data/pdoc-cache/` — **ruling (a)**: the 9,010 JSON records are **not** in git.
They ship as `takehome-conformance-corpus-2026.1.tar.zst` on the GitHub
release (`records/` + `pdoc-identity.json` + `screenshot-hashes.json`).
`pdoc-identity.json`, `screenshot-hashes.json`, and `corpus-archive.json`
stay in the repo. PNG screenshots stay local in `data/pdoc-screenshots/`
(gitignored). `CONFORMANCE.md` records the catalog digest and the archive
SHA-256.

Pack the archive after a capture (does not publish):

```bash
npm run record-screenshot-hashes
../../scripts/pack-conformance-corpus.sh
```

Key:
`sha256(canonical_input_json || rule_set_version)`.

Every record stores `observedEdition` (the calendar edition live PDOC was
serving at capture). A record may only satisfy a case whose `ruleSetVersion`
equals that edition **or** is proven identical for the case jurisdictions in
`data/edition-identity.json` (test 18 / test 19). Otherwise: hard error /
edition-retired.

A new run that observes a different PDOC identity than `pdoc-identity.json`
**exits non-zero** and does not drain the queue (§10.2).

Queue size is the distinct capturable July form count from
`takehome-grid-gen --queue` (ten of fourteen legal P). Uncapturable forms are
not skipped silently under an all-14-P headline — they are a named oracle
class in `CONFORMANCE.md`.
