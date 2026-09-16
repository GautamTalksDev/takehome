# netpay-grid-gen

Deterministic conformance case list (spec §11.2). Boundary density beats volume.

```bash
cargo run -p netpay-grid-gen -- --manifest
cargo run -p netpay-grid-gen -- --out /tmp/netpay-grid
cargo run -p netpay-grid-gen -- --report
```

`--out DIR` writes `manifest.json` and `cases.jsonl`. The bytes of that
emission are a pure function of `GRID_VERSION`, `SEED`, and the embedded
rule sets. Same seed, same grid, forever.

`--report` prints the oracle sampling census (unique PDOC forms vs interior
cells). The adopted split is documented in root `CONFORMANCE.md`.

Every conformance run must record `grid_version` from the manifest
(see root `CONFORMANCE.md`).
