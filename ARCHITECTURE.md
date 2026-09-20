# Architecture

**Who this is for:** Contributors wiring a new runtime, rule edition, or conformance class who need the system map before they touch code.

**When you finish:** You know where determinism is enforced, how rule data flows, and how the same engine reaches browser, Worker, and CLI.

## Engine contract

`takehome-core` is a pure function: given a [`Request`](crates/takehome-core/src/request.rs) and a loaded rule set, it returns a [`Response`](crates/takehome-core/src/response.rs) or a typed error. There is **no** network, clock, environment, or IO inside the library. The crate root documents this contract explicitly.

Hosted API and WASM wrappers may fetch rule JSON at load time. The core calculation path does not.

<figure>
  <img src="/diagrams/01-request-lifecycle.svg" alt="Request lifecycle" />
  <figcaption>One authenticated POST /v1/deductions from arrival to response. Branches mark failures and their HTTP statuses. The meter increments only after a successful calculation.</figcaption>
</figure>

## Rule data model

CRA publishes T4127 (Payroll Deductions Formulas) tables as CSV and PDF. Ingest (`tools/ingest`) archives sources with content hashes, parses rows, and emits versioned JSON under `data/rules/<effective-date>/`. Each **effective date** (calendar `as_of` on the request) selects a **rule set version** such as `2026-01-01`, `2026-07-01`, or proposed `2027-01-01`. July sets overlay January; they do not replace the whole tree.

Jurisdiction files (federal, provincial, CPP, and claim codes for TD1 federal and provincial personal amounts) load through `crates/takehome-core/src/rules/`. Vendored snapshots ship inside the crate at `crates/takehome-core/vendor/rules/` for offline WASM and deterministic tests. `RuleSetListing` exposes status (enacted vs proposed).

<figure>
  <img src="/diagrams/04-rule-data-pipeline.svg" alt="Rule data pipeline" />
  <figcaption>CRA publication is archived with a content hash, parsed, signed, and verified before load. The July document overlays the January set rather than replacing the whole tree.</figcaption>
</figure>

Diagram sources live in [`docs/diagrams/`](docs/diagrams/). Built SVGs are copied to `web/site/public/diagrams/`. Embed with the figure pattern in [`docs/diagrams/_include.md`](docs/diagrams/_include.md).

## Fixed-point money, never float

All money and rates enter through lexical strings (`Money::from_str`, `Rate::from_str`). IEEE-754 binary floats are forbidden in `takehome-core` (CI runs `./scripts/float-ban.sh`).

T4127 splits the annual basic exemption across pay periods with **truncate** to cents, not banker's round on that step. Example canary from M0:

```text
truncate_exemption_to_cent(3500 / 12) == 291.66
round_contribution_to_cent(3500 / 12) == 291.67
```

Every payable cent passes a named function in `crates/takehome-core/src/rounding.rs`. ADR-001 wraps `rust_decimal` behind newtypes so floats never enter the type surface.

## One Rust core, many runtimes

<figure>
  <img src="/diagrams/06-one-engine-every-runtime.svg" alt="One engine, every runtime" />
  <figcaption>The same Rust core compiles to native and to WASM. Browser, Node, Python, and the Worker share engine_build_sha256. The browser path makes no network call after load.</figcaption>
</figure>

- **Native tests and CLI (`takehome-core`, `takehome-cli`):** Vectors, property tests, operator tools.
- **WASM (`takehome-wasm`):** Browser embed and `takehome-ca` npm.
- **Node API (`services/api`, `engine-node.js`):** Cloudflare Worker handler.
- **Python (`packages/takehome-py`):** FFI to the same WASM/native artifact.

`Response` includes `ENGINE_BUILD_SHA256` so callers pin the binary they tested.

## Repository layout

```text
crates/takehome-core/     T4127 formulas, rounding, rules loader, vectors
crates/takehome-wasm/     wasm-bindgen exports
crates/takehome-cli/      command-line calculate and inspect
data/rules/               ingested rule JSON by effective date
data/sources/             archived CRA CSV inputs
tools/ingest/             parser and verifier
tools/grid-gen/           conformance grid generator
tools/conformance/        PDOC agreement report generator
tools/pdoc-oracle/        live PDOC capture harness
services/api/             Worker API, D1 store, Stripe
web/site/                 Astro docs, embed, public diagrams
packages/takehome-js/     npm wrapper
packages/takehome-py/     PyPI wrapper
CONFORMANCE.md            generated PDOC agreement (do not hand-edit counts)
docs/                     ADRs, findings, operator and contributor guides
```

Conformance and security docs: [`docs/conformance.md`](docs/conformance.md), [`docs/security.md`](docs/security.md).

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
