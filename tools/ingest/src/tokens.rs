//! Special tokens where a number is expected.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialToken {
    /// Federal BPAF formula.
    Bpaf,
    /// Manitoba BPAMB formula.
    Bpamb,
    /// Yukon BPAYT (mirrors BPAF).
    Bpayt,
    /// Claim code 0 / no personal amount.
    NoClaimAmount,
}

/// Map a CSV cell that is not a number. `None` means "not a special token".
pub fn special_token(raw: &str) -> Option<SpecialToken> {
    match raw.trim() {
        "BPAF" => Some(SpecialToken::Bpaf),
        "BPAMB" => Some(SpecialToken::Bpamb),
        "BPAYT" => Some(SpecialToken::Bpayt),
        "No claim amount" => Some(SpecialToken::NoClaimAmount),
        _ => None,
    }
}
