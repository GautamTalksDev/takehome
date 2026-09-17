# takehome-conformance

Generates root `CONFORMANCE.md` and `conformance.json` from the repo (M1
vectors, grid sampling, rule-set hash). Spec §11.4.

```bash
cargo run -p takehome-conformance -- --write
cargo run -p takehome-conformance -- --check
cargo run -p takehome-conformance -- --require-complete   # exits 1 while a PDOC corpus is pending
```

A partial run writes a **run in progress** document with **no overall
agreement rate**. CI runs `--check` so a rule-set change without regenerating
fails.
