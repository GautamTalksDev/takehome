//! Yukon K4P employment credit (T4127 Chapter 4).
//!
//! Same formula and newtype as federal K4: [`GrossEmploymentIncome`], never
//! annual taxable income (spec §21.2).

pub use crate::formulas::credits::k4p as yukon_k4p;
