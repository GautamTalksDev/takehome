//! Golden vector: T4127 Chapter 6 CPP basic exemption table (2026).
//!
//! Loads committed provenance-bearing JSON rather than inline constants.

use netpay_core::decimal::Ratio;
use netpay_core::rounding::truncate_exemption_to_cent;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    vectors: Vec<Vector>,
}

#[derive(Debug, Deserialize)]
struct Vector {
    input: Input,
    expected: String,
    source_document: String,
    source_url: String,
}

#[derive(Debug, Deserialize)]
struct Input {
    annual_exemption: String,
    pay_periods: String,
}

#[test]
fn cpp_exemption_table_matches_committed_golden_vectors() {
    let raw = include_str!("vectors/cpp_exemption_2026.json");
    let fixture: Fixture = serde_json::from_str(raw).expect("fixture JSON parses");
    assert_eq!(
        fixture.vectors.len(),
        14,
        "fixture must carry all 14 pay-period rows"
    );

    for row in &fixture.vectors {
        assert!(
            !row.source_document.is_empty() && !row.source_url.is_empty(),
            "every golden vector carries provenance"
        );
        let ratio =
            Ratio::div(&row.input.annual_exemption, &row.input.pay_periods).unwrap_or_else(|e| {
                panic!(
                    "Ratio::div({}, {}): {e}",
                    row.input.annual_exemption, row.input.pay_periods
                )
            });
        let got = truncate_exemption_to_cent(ratio);
        assert_eq!(
            got.to_string(),
            row.expected,
            "P={}: {} / {} (from {})",
            row.input.pay_periods,
            row.input.annual_exemption,
            row.input.pay_periods,
            row.source_document
        );
    }

    // Discriminating canary retained at the integration boundary.
    let monthly = Ratio::div("3500.00", "12").unwrap();
    assert_eq!(truncate_exemption_to_cent(monthly).to_string(), "291.66");
}
