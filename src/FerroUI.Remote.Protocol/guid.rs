//! A globally unique identifier, with the text forms and the byte layout of
//! the `System.Guid` the protocol is written against.
//!
//! The byte layout is part of the wire format: the header of every message
//! carries the sixteen bytes of [`Guid::to_byte_array`], in which the first
//! three groups of the text form are little-endian and the last two are in
//! the order of the text.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::error::Error;

/// A 128-bit identifier.
///
/// It is a [`Uuid`] of the `uuid` crate, the identifier type of the workspace
/// (docs/idiomatic-rust.md): the two convert into each other with `From`, and
/// the layout of [`Guid::to_byte_array`] is [`Uuid::to_bytes_le`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(Uuid);

impl Guid {
    /// The identifier whose bits are all zero.
    pub const EMPTY: Guid = Guid(Uuid::nil());

    /// The identifier with the given groups.
    pub const fn new(a: u32, b: u16, c: u16, d: [u8; 8]) -> Guid {
        Guid(Uuid::from_fields(a, b, c, &d))
    }

    /// Parses the forms `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` and the same
    /// thirty-two digits without hyphens, either of them optionally in braces
    /// or parentheses. The digits may be of either case.
    const fn try_parse_bytes(s: &[u8]) -> Option<Guid> {
        let mut inner = s;
        if s.len() >= 2 {
            if let (b'{', b'}') | (b'(', b')') = (s[0], s[s.len() - 1]) {
                let (_, rest) = s.split_at(1);
                let (middle, _) = rest.split_at(rest.len() - 1);
                inner = middle;
            }
        }
        // `Uuid` reads 32 digits, or 36 characters with the hyphens in their places; its other
        // forms (a URN, braces of its own) are not forms of `Guid.Parse` and are left out here.
        match inner.len() {
            32 | 36 => match Uuid::try_parse_ascii(inner) {
                Ok(uuid) => Some(Guid(uuid)),
                Err(_) => None,
            },
            _ => None,
        }
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
        let bytes: [u8; 16] = bytes
            .try_into()
            .map_err(|_| Error::Argument("Byte array for Guid must be exactly 16 bytes long.".to_string()))?;
        Ok(Guid(Uuid::from_bytes_le(bytes)))
    }

    /// `Guid.ToByteArray`: the first three groups little-endian, then the
    /// last eight bytes in the order of the text.
    pub fn to_byte_array(&self) -> [u8; 16] {
        self.0.to_bytes_le()
    }
}

impl From<Uuid> for Guid {
    fn from(value: Uuid) -> Self {
        Guid(value)
    }
}

impl From<Guid> for Uuid {
    fn from(value: Guid) -> Self {
        value.0
    }
}

/// `Guid.ToString()`: the hyphenated form in lower case.
impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.hyphenated(), f)
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
        assert!(matches!(Guid::from_byte_array(&[0; 17]), Err(Error::Argument(_))));
        // Forms of `Uuid` that are not forms of `Guid.Parse`.
        assert!(matches!(Guid::parse("urn:uuid:6e3c5310-e2b1-4c3d-8688-01183aa48c5b"), Err(Error::Format(_))));
        assert!(matches!(Guid::parse("6E3C5310-E2B14C3D-8688-01183AA48C5B-"), Err(Error::Format(_))));
    }

    #[test]
    fn the_groups_and_the_order_are_those_of_the_text() {
        let guid = Guid::new(0x6E3C5310, 0xE2B1, 0x4C3D, [0x86, 0x88, 0x01, 0x18, 0x3A, 0xA4, 0x8C, 0x5B]);
        assert_eq!(guid, Guid::parse_const("6E3C5310-E2B1-4C3D-8688-01183AA48C5B"));
        assert_eq!(Guid::parse("(6E3C5310-E2B1-4C3D-8688-01183AA48C5B)").unwrap(), guid);
        assert_eq!(Guid::parse("{6E3C5310E2B14C3D868801183AA48C5B}").unwrap(), guid);
        assert_eq!(Uuid::from(guid).to_string(), guid.to_string());
        assert_eq!(Guid::from(Uuid::from(guid)), guid);
        assert_eq!(Guid::EMPTY, Guid::default());
        assert!(Guid::new(1, 0, 0, [0; 8]) > Guid::new(0, 0xFFFF, 0xFFFF, [0xFF; 8]));
        assert!(Guid::new(0, 0, 0, [0, 0, 0, 0, 0, 0, 0, 1]) > Guid::EMPTY);
    }
}
