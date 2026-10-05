// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use super::encoding_ids::EncodingIDs;

/// The text encodings the table readers decode (the counterpart of the
/// `System.Text.Encoding` instances the reference implementation selects).
///
/// Malformed input never fails: invalid sequences decode to U+FFFD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-16, big-endian.
    BigEndianUnicode,
    /// UTF-8.
    UTF8,
}

impl Encoding {
    /// Decodes `bytes`.
    pub fn get_string(&self, bytes: &[u8]) -> String {
        match self {
            Encoding::BigEndianUnicode => {
                let pairs = bytes.chunks_exact(2);
                let has_trailing_byte = !pairs.remainder().is_empty();
                let units = pairs.map(|pair| u16::from_be_bytes([pair[0], pair[1]]));

                let mut text: String = char::decode_utf16(units)
                    .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
                    .collect();

                // A dangling odd byte is an incomplete code unit.
                if has_trailing_byte {
                    text.push(char::REPLACEMENT_CHARACTER);
                }

                text
            }
            Encoding::UTF8 => String::from_utf8_lossy(bytes).into_owned(),
        }
    }
}

/// Converts encoding ID to text encoding.
pub struct EncodingIDExtensions;

impl EncodingIDExtensions {
    /// Converts encoding ID to text encoding.
    pub fn as_encoding(id: EncodingIDs) -> Encoding {
        if id == EncodingIDs::Unicode11 || id == EncodingIDs::Unicode2 {
            Encoding::BigEndianUnicode
        } else {
            Encoding::UTF8
        }
    }
}

impl EncodingIDs {
    /// Converts encoding ID to text encoding.
    #[inline]
    pub fn as_encoding(self) -> Encoding {
        EncodingIDExtensions::as_encoding(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_utf16_for_windows_unicode_encodings() {
        assert_eq!(EncodingIDs::Unicode11.as_encoding(), Encoding::BigEndianUnicode);
        assert_eq!(EncodingIDs::Unicode2.as_encoding(), Encoding::BigEndianUnicode);
        assert_eq!(EncodingIDs::Unicode1.as_encoding(), Encoding::UTF8);
        assert_eq!(EncodingIDs(77).as_encoding(), Encoding::UTF8);
    }

    #[test]
    fn decodes_utf16_big_endian() {
        // "A", U+1F600 (surrogate pair), then a lone high surrogate and a dangling byte.
        let bytes = [0x00, 0x41, 0xD8, 0x3D, 0xDE, 0x00, 0xD8, 0x00, 0x42];

        assert_eq!(Encoding::BigEndianUnicode.get_string(&bytes), "A\u{1F600}\u{FFFD}\u{FFFD}");
        assert_eq!(Encoding::BigEndianUnicode.get_string(&[]), "");
    }

    #[test]
    fn decodes_utf8_with_replacement() {
        assert_eq!(Encoding::UTF8.get_string(b"Inter"), "Inter");
        assert_eq!(Encoding::UTF8.get_string(&[b'a', 0xFF, b'b']), "a\u{FFFD}b");
    }
}
