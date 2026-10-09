//! A globally unique identifier, with the text forms and the byte layout of
//! the `System.Guid` the protocol is written against.
//!
//! The byte layout is part of the wire format: the header of every message
//! carries the sixteen bytes of [`Guid::to_byte_array`], in which the first
//! three groups of the text form are little-endian and the last two are in
//! the order of the text.

use std::fmt;
use std::str::FromStr;

use crate::error::Error;

/// A 128-bit identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}

const fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl Guid {
    /// The identifier whose bits are all zero.
    pub const EMPTY: Guid = Guid { a: 0, b: 0, c: 0, d: [0; 8] };

    /// The identifier with the given groups.
    pub const fn new(a: u32, b: u16, c: u16, d: [u8; 8]) -> Guid {
        Guid { a, b, c, d }
    }

    /// Parses the forms `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` and the same
    /// thirty-two digits without hyphens, either of them optionally in braces
    /// or parentheses. The digits may be of either case.
    const fn try_parse_bytes(s: &[u8]) -> Option<Guid> {
        let mut start = 0;
        let mut end = s.len();
        if end >= 2 && ((s[0] == b'{' && s[end - 1] == b'}') || (s[0] == b'(' && s[end - 1] == b')')) {
            start = 1;
            end -= 1;
        }
        let hyphens = match end - start {
            36 => true,
            32 => false,
            _ => return None,
        };
        let mut digits = [0u8; 32];
        let mut count = 0;
        let mut i = start;
        while i < end {
            let position = i - start;
            if hyphens && (position == 8 || position == 13 || position == 18 || position == 23) {
                if s[i] != b'-' {
                    return None;
                }
            } else {
                match hex_digit(s[i]) {
                    Some(digit) => {
                        if count == 32 {
                            return None;
                        }
                        digits[count] = digit;
                        count += 1;
                    }
                    None => return None,
                }
            }
            i += 1;
        }
        if count != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        let mut k = 0;
        while k < 16 {
            bytes[k] = (digits[2 * k] << 4) | digits[2 * k + 1];
            k += 1;
        }
        Some(Guid {
            a: u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            b: u16::from_be_bytes([bytes[4], bytes[5]]),
            c: u16::from_be_bytes([bytes[6], bytes[7]]),
            d: [bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]],
        })
    }

    /// `Guid.Parse` for a constant: a text that is not an identifier fails
    /// the build.
    pub const fn parse_const(text: &str) -> Guid {
        match Guid::try_parse_bytes(text.as_bytes()) {
            Some(guid) => guid,
            None => panic!("Unrecognized Guid format."),
        }
    }

    /// `Guid.Parse`: a `FormatException` is [`Error::Format`].
    pub fn parse(text: &str) -> Result<Guid, Error> {
        Guid::try_parse_bytes(text.trim().as_bytes())
            .ok_or_else(|| Error::Format("Unrecognized Guid format.".to_string()))
    }

    /// `new Guid(byte[])`: the layout of [`Guid::to_byte_array`]. Anything
    /// but sixteen bytes is an `ArgumentException`.
    pub fn from_byte_array(bytes: &[u8]) -> Result<Guid, Error> {
        if bytes.len() != 16 {
            return Err(Error::Argument("Byte array for Guid must be exactly 16 bytes long.".to_string()));
        }
        let mut d = [0u8; 8];
        d.copy_from_slice(&bytes[8..16]);
        Ok(Guid {
            a: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            b: u16::from_le_bytes([bytes[4], bytes[5]]),
            c: u16::from_le_bytes([bytes[6], bytes[7]]),
            d,
        })
    }

    /// `Guid.ToByteArray`: the first three groups little-endian, then the
    /// last eight bytes in the order of the text.
    pub fn to_byte_array(&self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[0..4].copy_from_slice(&self.a.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.b.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.c.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.d);
        bytes
    }
}

/// `Guid.ToString()`: the hyphenated form in lower case.
impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.a,
            self.b,
            self.c,
            self.d[0],
            self.d[1],
            self.d[2],
            self.d[3],
            self.d[4],
            self.d[5],
            self.d[6],
            self.d[7]
        )
    }
}

impl FromStr for Guid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Guid::parse(s)
    }
}

// Tests of the port: the upstream type is the one of the runtime library.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_the_hyphenated_form() {
        let guid = Guid::parse("6E3C5310-E2B1-4C3D-8688-01183AA48C5B").unwrap();
        assert_eq!(guid.to_string(), "6e3c5310-e2b1-4c3d-8688-01183aa48c5b");
        assert_eq!(guid, Guid::parse_const("6e3c5310-e2b1-4c3d-8688-01183aa48c5b"));
        assert_eq!(guid, Guid::parse("{6E3C5310-E2B1-4C3D-8688-01183AA48C5B}").unwrap());
        assert_eq!(guid, Guid::parse("6E3C5310E2B14C3D868801183AA48C5B").unwrap());
        assert_eq!(guid, "6E3C5310-E2B1-4C3D-8688-01183AA48C5B".parse::<Guid>().unwrap());
    }

    #[test]
    fn byte_array_has_the_first_three_groups_little_endian() {
        let guid = Guid::parse_const("6E3C5310-E2B1-4C3D-8688-01183AA48C5B");
        let bytes = guid.to_byte_array();
        assert_eq!(
            bytes,
            [0x10, 0x53, 0x3C, 0x6E, 0xB1, 0xE2, 0x3D, 0x4C, 0x86, 0x88, 0x01, 0x18, 0x3A, 0xA4, 0x8C, 0x5B]
        );
        assert_eq!(Guid::from_byte_array(&bytes).unwrap(), guid);
    }

    #[test]
    fn rejects_what_is_not_an_identifier() {
        assert!(matches!(Guid::parse(""), Err(Error::Format(_))));
        assert!(matches!(Guid::parse("6E3C5310-E2B1-4C3D-8688-01183AA48C5"), Err(Error::Format(_))));
        assert!(matches!(Guid::parse("6E3C5310+E2B1-4C3D-8688-01183AA48C5B"), Err(Error::Format(_))));
        assert!(matches!(Guid::parse("6E3C5310-E2B1-4C3D-8688-01183AA48C5G"), Err(Error::Format(_))));
        assert!(matches!(Guid::from_byte_array(&[0; 15]), Err(Error::Argument(_))));
    }
}
