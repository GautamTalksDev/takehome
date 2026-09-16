//! Per-file encoding detection. Never assume UTF-8.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingKind {
    Utf8,
    Utf8Bom,
    Windows1252,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub encoding: EncodingKind,
    pub text: String,
}

/// Decode archived CSV bytes. UTF-8 (with or without BOM) if valid; otherwise
/// Windows-1252. Detection is per buffer, never inherited from another file.
pub fn decode_bytes(bytes: &[u8]) -> Decoded {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let rest = &bytes[3..];
        if let Ok(text) = std::str::from_utf8(rest) {
            return Decoded {
                encoding: EncodingKind::Utf8Bom,
                text: text.to_string(),
            };
        }
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Decoded {
            encoding: EncodingKind::Utf8,
            text: text.to_string(),
        };
    }
    Decoded {
        encoding: EncodingKind::Windows1252,
        text: windows_1252_to_string(bytes),
    }
}

fn windows_1252_to_string(bytes: &[u8]) -> String {
    bytes.iter().copied().map(windows_1252_char).collect()
}

fn windows_1252_char(byte: u8) -> char {
    match byte {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        other => char::from(other),
    }
}
