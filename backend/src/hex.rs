//! Hex decoding for the MACs and signatures the backend hands out and checks (Stripe
//! webhooks, store cancel tokens, upload URLs).

/// The bytes a hex string encodes, in either case. `None` for an odd length or a
/// non-hex character.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes() {
        assert_eq!(decode("00ff"), Some(vec![0, 255]));
        assert_eq!(decode("00FF"), Some(vec![0, 255]));
        assert_eq!(decode(""), Some(vec![]));
        assert_eq!(decode("0"), None);
        assert_eq!(decode("zz"), None);
        assert_eq!(decode("é0"), None, "a multi-byte character isn't hex");
    }
}
