//! WASM surface. Core stays IO-free; this crate only re-exports JSON listings.

/// `GET /v1/jurisdictions` body: supported codes plus Quebec as unsupported.
pub fn jurisdictions() -> String {
    netpay_core::jurisdictions_json()
}

pub fn placeholder() {}
