//! CRA T4127 payroll deduction formulas — core library (`netpay-core`).
//!
//! # Determinism contract
//!
//! This crate is pure computation: **no I/O, no clock, no network, and no
//! environment reads**. Outputs depend only on the values passed in. Randomness
//! and wall-clock time are forbidden. Callers that need files, HTTP, or the
//! system clock keep that outside this crate.
#![deny(clippy::float_arithmetic)]

pub mod decimal;
pub mod rounding;

pub use decimal::{DecimalError, Money, Rate, Ratio};
