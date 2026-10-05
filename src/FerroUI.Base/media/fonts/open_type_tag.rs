use std::fmt;

/// A four byte OpenType tag (table, script, feature or axis tag).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OpenTypeTag {
    value: u32,
}

impl OpenTypeTag {
    pub(crate) const NONE: OpenTypeTag = OpenTypeTag::from_bytes(0, 0, 0, 0);
    pub(crate) const MAX: OpenTypeTag = OpenTypeTag::from_bytes(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    pub(crate) const MAX_SIGNED: OpenTypeTag = OpenTypeTag::from_bytes(i8::MAX as u8, u8::MAX, u8::MAX, u8::MAX);

    /// Creates a tag from its packed big-endian value.
    pub const fn new(value: u32) -> Self {
        Self { value }
    }

    /// Creates a tag from four characters; only the low byte of each is used.
    pub const fn from_chars(c1: char, c2: char, c3: char, c4: char) -> Self {
        Self::from_bytes(c1 as u32 as u8, c2 as u32 as u8, c3 as u32 as u8, c4 as u32 as u8)
    }

    /// Creates a tag from four bytes.
    pub const fn from_bytes(c1: u8, c2: u8, c3: u8, c4: u8) -> Self {
        Self { value: ((c1 as u32) << 24) | ((c2 as u32) << 16) | ((c3 as u32) << 8) | c4 as u32 }
    }

    /// Parses a tag from a string: at most four UTF-16 code units are used and
    /// short tags are padded with spaces. An empty string gives the `None` tag.
    pub fn parse(tag: &str) -> Self {
        if tag.is_empty() {
            return Self::NONE;
        }

        let mut real_tag = [b' '; 4];

        for (i, unit) in tag.encode_utf16().take(4).enumerate() {
            real_tag[i] = unit as u8;
        }

        Self::from_bytes(real_tag[0], real_tag[1], real_tag[2], real_tag[3])
    }

    /// The packed big-endian value.
    #[inline]
    pub const fn value(self) -> u32 {
        self.value
    }
}

impl fmt::Display for OpenTypeTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::NONE {
            return f.write_str("None");
        }
        if *self == Self::MAX {
            return f.write_str("Max");
        }
        if *self == Self::MAX_SIGNED {
            return f.write_str("MaxSigned");
        }

        for shift in [24u32, 16, 8, 0] {
            // Each byte is one Latin-1 character, as upstream's `(char)(byte)` cast.
            fmt::Write::write_char(f, char::from((self.value >> shift) as u8))?;
        }

        Ok(())
    }
}

impl From<OpenTypeTag> for u32 {
    fn from(tag: OpenTypeTag) -> u32 {
        tag.value
    }
}

impl From<u32> for OpenTypeTag {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pads_and_displays() {
        assert_eq!(OpenTypeTag::parse("cmap").value(), 0x636D_6170);
        assert_eq!(OpenTypeTag::parse("cmap").to_string(), "cmap");
        assert_eq!(OpenTypeTag::parse("CFF").to_string(), "CFF ");
        assert_eq!(OpenTypeTag::parse(""), OpenTypeTag::NONE);
        assert_eq!(OpenTypeTag::NONE.to_string(), "None");
        assert_eq!(OpenTypeTag::from_chars('k', 'e', 'r', 'n'), OpenTypeTag::parse("kern"));
    }
}
