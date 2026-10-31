use std::fmt;

use uuid::Uuid;

/// Binary-compatible with the `GUID` struct declared in `com.h`.
///
/// The layout is the one of the system; the value is a [`Uuid`] of the `uuid` crate, the
/// identifier type of the workspace (docs/idiomatic-rust.md), and the two convert into each other
/// with `From`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    pub const ZERO: Guid = Guid::new(0, 0, 0, [0; 8]);

    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Guid { data1, data2, data3, data4 }
    }

    /// Builds a GUID from its 128-bit big-endian textual value, e.g.
    /// `Guid::from_u128(0x809c652e_7396_11d2_9771_00a0c9b4d50c)`.
    pub const fn from_u128(v: u128) -> Self {
        let b = v.to_be_bytes();
        Guid {
            data1: u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            data2: u16::from_be_bytes([b[4], b[5]]),
            data3: u16::from_be_bytes([b[6], b[7]]),
            data4: [b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]],
        }
    }

    /// Parses `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, optionally in one pair of braces, with white
    /// space around it.
    pub fn parse(s: &str) -> Option<Guid> {
        let s = s.trim();
        let s = s.strip_prefix('{').and_then(|inner| inner.strip_suffix('}')).unwrap_or(s);
        if s.len() != 36 {
            return None;
        }
        Uuid::try_parse(s).ok().map(Guid::from)
    }

    /// The value native libraries built before the `com.h` fix materialize for this IID.
    ///
    /// The `__IID_DEF` macro of `com.h` used to initialize `Data4` as
    /// `{d41, d42, d42, d42, d42, d42, d42, d42}`: the second byte was repeated
    /// instead of using bytes 3..8. Native `QueryInterface` implementations
    /// `memcmp` against those constants, so an older native library answers this
    /// form; current ones answer the IDL value. Both are accepted and tried.
    pub const fn as_com_h_materialized(&self) -> Guid {
        let d = self.data4;
        Guid {
            data1: self.data1,
            data2: self.data2,
            data3: self.data3,
            data4: [d[0], d[1], d[1], d[1], d[1], d[1], d[1], d[1]],
        }
    }
}

impl From<Uuid> for Guid {
    fn from(value: Uuid) -> Self {
        let (data1, data2, data3, data4) = value.as_fields();
        Guid { data1, data2, data3, data4: *data4 }
    }
}

impl From<Guid> for Uuid {
    fn from(value: Guid) -> Self {
        Uuid::from_fields(value.data1, value.data2, value.data3, &value.data4)
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&Uuid::from(*self).hyphenated(), f)
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({self})")
    }
}
