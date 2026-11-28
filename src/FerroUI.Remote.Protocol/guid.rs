//! The identifiers of the protocol: [`Uuid`]s, with the text forms and the
//! byte layout of the `System.Guid` the protocol is written against.
//!
//! The byte layout is part of the wire format: the header of every message
//! carries the sixteen bytes of [`Uuid::to_bytes_le`] (`Guid.ToByteArray`),
//! in which the first three groups of the text form are little-endian and
//! the last two are in the order of the text. The serializer writes and
//! reads that layout; the public signatures of the library take and return
//! `Uuid`.

use uuid::Uuid;

use crate::error::Error;

/// `Guid.Parse` of the forms `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` and the
/// same thirty-two digits without hyphens, either of them optionally in
/// braces or parentheses. The digits may be of either case.
pub(crate) const fn try_parse_uuid(s: &[u8]) -> Option<Uuid> {
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
            Ok(uuid) => Some(uuid),
            Err(_) => None,
        },
        _ => None,
    }
}

/// `Guid.Parse` for a constant: a text that is not an identifier fails the
/// build.
pub(crate) const fn parse_uuid_const(text: &str) -> Uuid {
    match try_parse_uuid(text.as_bytes()) {
        Some(uuid) => uuid,
        None => panic!("Unrecognized Guid format."),
    }
}

/// `new Guid(byte[])` at the serializer: the layout of
/// [`Uuid::from_bytes_le`]. Anything but sixteen bytes is an
/// `ArgumentException`.
pub(crate) fn uuid_from_byte_array(bytes: &[u8]) -> Result<Uuid, Error> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| Error::Argument("Byte array for Guid must be exactly 16 bytes long.".to_string()))?;
    Ok(Uuid::from_bytes_le(bytes))
}

// Tests of the port: the upstream type is the one of the runtime library.
