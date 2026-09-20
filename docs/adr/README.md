# Architecture decision records

**Who this is for:** Contributors who need the short index before reading a full ADR.

**When you finish:** You know which decision covers decimals, K constants, PDOC rounding, API keys, and metering.

Full text lives beside this folder as `docs/ADR-001-decimal.md` through
`docs/ADR-006-billing-deferred.md`. Thin stubs in this folder point to those
files.

- **[ADR-001](ADR-001.md):** Wrap `rust_decimal` in string-constructed newtypes; no floats in the engine.
- **[ADR-002](ADR-002.md):** Use CRA published whole-dollar K; accept bracket threshold discontinuity.
- **[ADR-003](ADR-003.md):** Reserve `rounding_compat` (`t4127` default; `pdoc` fails closed until M-003 closes).
- **[ADR-004](ADR-004.md):** Store API key SHA-256 digests, not password KDFs.
- **[ADR-005](ADR-005.md):** Refuse calculations when D1 metering read/write fails.
- **[ADR-006](ADR-006.md):** Defer Stripe billing for free early access; withdraw kill-test route C.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
