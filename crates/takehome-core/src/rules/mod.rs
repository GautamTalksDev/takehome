//! Effective-date versioned T4127 rule data.
//!
//! The engine is given a [`schema::RuleSet`] and a request; it does not load
//! files or consult a clock.

pub mod diff;
pub mod loader;
pub mod registry;
pub mod schema;
