//! Exact-K identity for the published whole-dollar K canary (ADR-002).
//!
//! The engine uses CRA's published K. These helpers reconstruct the unrounded
//! Σ threshold[j]×(rate[j]−rate[j−1]) so tests can pin the rounding residual
//! instead of demanding an equality that the published table does not satisfy.

use crate::decimal::{DecimalError, Money};
use crate::rules::schema::Bracket;

/// Unrounded K for each bracket: `K_exact[0] = 0`, and
/// `K_exact[i] = Σ_{j=1..i} threshold[j] × (rate[j] − rate[j−1])`.
pub(crate) fn k_exact_constants(brackets: &[Bracket]) -> Result<Vec<Money>, DecimalError> {
    let mut out = Vec::with_capacity(brackets.len());
    out.push(Money::ZERO);
    let mut cumulative = Money::ZERO;
    for i in 1..brackets.len() {
        let t = brackets[i].threshold;
        let high = t.checked_mul_rate(brackets[i].rate)?;
        let low = t.checked_mul_rate(brackets[i - 1].rate)?;
        cumulative = cumulative.checked_add(high.checked_sub(low)?)?;
        out.push(cumulative);
    }
    Ok(out)
}

/// `K_exact − K_published` for one bracket (full internal scale).
pub(crate) fn k_residual(k_exact: Money, k_published: Money) -> Result<Money, DecimalError> {
    k_exact.checked_sub(k_published)
}

/// Independent marginal-slice sum of A (unrounded). Not the engine path.
pub(crate) fn marginal_slice_sum(a: Money, brackets: &[Bracket]) -> Result<Money, DecimalError> {
    let mut slices = Money::ZERO;
    for i in 0..brackets.len() {
        let lo = brackets[i].threshold;
        if a < lo {
            break;
        }
        let top = if i + 1 < brackets.len() {
            brackets[i + 1].threshold.min(a)
        } else {
            a
        };
        if top <= lo {
            continue;
        }
        let width = top.checked_sub(lo)?;
        slices = slices.checked_add(width.checked_mul_rate(brackets[i].rate)?)?;
    }
    Ok(slices)
}

/// Unrounded `(R × A) − K_published` for the occupied bracket.
pub(crate) fn ra_minus_published_k(a: Money, brackets: &[Bracket]) -> Result<Money, DecimalError> {
    let bracket = brackets
        .iter()
        .rev()
        .find(|b| a >= b.threshold)
        .expect("bracket table starts at threshold 0");
    a.checked_mul_rate(bracket.rate)?
        .checked_sub(bracket.constant)
}

/// Discontinuity at threshold `i>0`:
/// `(K_exact,i − K_published,i) − (K_exact,i−1 − K_published,i−1)`.
pub(crate) fn k_rounding_discontinuity(
    brackets: &[Bracket],
    index: usize,
) -> Result<Money, DecimalError> {
    let exact = k_exact_constants(brackets)?;
    let res_i = k_residual(exact[index], brackets[index].constant)?;
    let res_prev = k_residual(exact[index - 1], brackets[index - 1].constant)?;
    res_i.checked_sub(res_prev)
}
