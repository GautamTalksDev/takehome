# ADR-003: `rounding_compat` request field

**Who this is for:** API and engine contributors handling M-003 PDOC cent deltas.

**When you finish:** You know the default compat mode and why `pdoc` fails closed today.

**Status:** Accepted (schema reserved; `pdoc` is a typed not-implemented error until finding 002 closes).  
**Date:** 2026-09-12

## Context

M-002 (`k2_method`) taught the pattern: when PDOC and T4127 (Payroll Deductions Formulas) diverge, customers
who reconcile against PDOC need a selectable arm, and the default should be
stated explicitly.

M-003 is a live cent delta where T4127 half-up at an exact midpoint goes up
and PDOC's corresponding line is sometimes 1¢ lower. It is not
Alberta-specific. The condition that selects down vs up is unnamed. A payroll
product that differs from PDOC by a predictable cent still needs an escape
hatch once that condition is known.

Request-schema fields are expensive after v1.

## Decision

1. Add request field **`rounding_compat`** with wire values:
   - **`t4127`** (default): exact decimal half-up for period tax lines
     (`round_tax_to_cent` on `T1/P` and `T2/P` as today).
   - **`pdoc`**: match live PDOC period federal/provincial lines when they
     diverge from `t4127` on a documented class.

2. **Default is `t4127`**, not `pdoc`. Unlike `k2_method` (where PDOC's rule
   was inverted and confirmed), we do not yet have a named midpoint condition
   to observe. Defaulting to an unimplemented or speculative `pdoc` arm would
   encode a guess. When finding 002 closes with a PDOC-observed rule, revisit
   whether the default should flip (as K2 did).

3. **Do not implement `pdoc` as IEEE-float residue.** Probes showed PDOC
   matches decimal half-up on most discriminating half-cent forms outside the
   down-classes. Float-compat would be the wrong product.

4. Until finding 002 names the midpoint condition, `rounding_compat: pdoc` is
   accepted on the wire and **`calculate` returns typed
   `RequestError::RoundingCompatPdocNotImplemented`**. A silent alias of
   `t4127` is a documented promise the engine does not keep. The field stays
   so the schema does not churn when the arm lands; the arm must not succeed
   until it is real.

## Consequences

- M-003 disagreements continue to count in the agreement rate under `t4127`.
- `rounding_compat: pdoc` is a legal request that **fails closed**. Customers
  who need bit-for-bit PDOC on midpoint-down forms wait until finding 002
  closes; they cannot build against a no-op arm.
- Grid / harness compare against PDOC under whatever compat the vector
  requests; default vectors use `t4127`.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
