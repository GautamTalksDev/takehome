# PRE-REGISTERED STOP CONDITION

Written before any engine code was committed. Commit date is the record.

Within 90 days of the API going live, at least ONE of:

  A. Twenty-five distinct domains have made a successful, non-test,
     authenticated API call, excluding CI and excluding anyone I
     personally know.

  B. The calculator pages record three thousand organic search
     sessions in a single calendar month.

  C. Three paying customers on any tier above free.

If none of the three fire, the numbers are published as they stand and
the hosted service is shut down. The engine and the rule data remain
published under their open licences.

API live date: <FILL IN>
Day 90: <FILL IN>

## Amendment, 2026-09-20

**Reason:** Paid billing is deferred for the free early-access launch
(ADR-006). Stripe checkout is not live. There is no paid tier a stranger
can buy.

**Change:** Route **C is withdrawn**. Routes **A and B stand unchanged**.
The stop condition is now: within 90 days of the API going live, at least
ONE of A or B. If neither fires, the numbers are published as they stand
and the hosted service is shut down. The engine and the rule data remain
published under their open licences.

The original three-route text above is kept as the pre-amendment record.
Do not silently restore C without a dated amendment that restores paid
billing first.
