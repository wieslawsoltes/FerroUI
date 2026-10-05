/// Helpers for key symbols reported by platform backends.
pub struct KeySymbolHelper;

impl KeySymbolHelper {
    /// Whether an ASCII character is acceptable as a key symbol: control
    /// characters other than backspace, tab, return and escape are not.
    pub fn is_allowed_ascii_key_symbol(c: char) -> bool {
        let code = c as u32;

        if code < 0x20 {
            // backspace, tab, return, escape
            return matches!(code, 0x08 | 0x09 | 0x0D | 0x1B);
        }

        // delete
        code != 0x7F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_control_characters() {
        assert!(KeySymbolHelper::is_allowed_ascii_key_symbol('a'));
        assert!(KeySymbolHelper::is_allowed_ascii_key_symbol('\t'));
        assert!(KeySymbolHelper::is_allowed_ascii_key_symbol('\u{1B}'));
        assert!(!KeySymbolHelper::is_allowed_ascii_key_symbol('\u{1}'));
        assert!(!KeySymbolHelper::is_allowed_ascii_key_symbol('\u{7F}'));
    }
}
