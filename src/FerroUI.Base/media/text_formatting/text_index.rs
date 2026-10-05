//! Conversion between UTF-16 code unit indices (the index unit of the text
//! subsystem) and UTF-8 byte offsets of a `&str`.

/// Converts a UTF-16 code unit index into `text` to the UTF-8 byte offset of
/// the same position. An index inside a surrogate pair maps to the start of
/// that character; an index past the end maps to `text.len()`.
pub fn utf16_index_to_utf8(text: &str, utf16_index: usize) -> usize {
    let mut units = 0usize;

    for (byte_offset, c) in text.char_indices() {
        if units + c.len_utf16() > utf16_index {
            return byte_offset;
        }
        units += c.len_utf16();
    }

    text.len()
}

/// Converts a UTF-8 byte offset into `text` to the UTF-16 code unit index of
/// the same position. An offset inside a character maps to the start of that
/// character; an offset past the end maps to the UTF-16 length.
pub fn utf8_index_to_utf16(text: &str, utf8_index: usize) -> usize {
    let mut units = 0usize;

    for (byte_offset, c) in text.char_indices() {
        if byte_offset + c.len_utf8() > utf8_index {
            return units;
        }
        units += c.len_utf16();
    }

    units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_units() {
        let text = "a\u{00E9}\u{1F600}b"; // 1 + 2 + 4 + 1 bytes; 1 + 1 + 2 + 1 units
        assert_eq!(utf16_index_to_utf8(text, 0), 0);
        assert_eq!(utf16_index_to_utf8(text, 1), 1);
        assert_eq!(utf16_index_to_utf8(text, 2), 3);
        assert_eq!(utf16_index_to_utf8(text, 3), 3); // inside the surrogate pair
        assert_eq!(utf16_index_to_utf8(text, 4), 7);
        assert_eq!(utf16_index_to_utf8(text, 5), 8);
        assert_eq!(utf16_index_to_utf8(text, 99), 8);

        assert_eq!(utf8_index_to_utf16(text, 0), 0);
        assert_eq!(utf8_index_to_utf16(text, 1), 1);
        assert_eq!(utf8_index_to_utf16(text, 2), 1); // inside "é"
        assert_eq!(utf8_index_to_utf16(text, 3), 2);
        assert_eq!(utf8_index_to_utf16(text, 7), 4);
        assert_eq!(utf8_index_to_utf16(text, 8), 5);
    }
}
