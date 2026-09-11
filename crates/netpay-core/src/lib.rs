//! netpay-core: no IO, no network, no clock, no environment access.
//! Given a Request and a RuleSet, returns a Response. Deterministic, forever.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

pub mod decimal;
pub mod rounding;

pub use decimal::{DecimalError, Money, Rate, Ratio};
