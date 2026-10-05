use std::fmt;

/// Binary-compatible with the `GUID` struct declared in `com.h`.
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

    /// Parses `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (braces optional).
    pub fn parse(s: &str) -> Option<Guid> {
        let s = s.trim().trim_start_matches('{').trim_end_matches('}');
        let hex: String = s.chars().filter(|c| *c != '-').collect();
        if hex.len() != 32 || s.len() != 36 {
            return None;
        }
        u128::from_str_radix(&hex, 16).ok().map(Guid::from_u128)
    }

    /// The value the native library actually materializes for this IID.
    ///
    /// `com.h`'s `__IID_DEF` macro (kept as it is in the native sources) initializes
    /// `Data4` as `{d41, d42, d42, d42, d42, d42, d42, d42}` — the second
    /// byte is repeated instead of using bytes 3..8. Native `QueryInterface`
    /// implementations `memcmp` against those constants, so when talking to
    /// the native side this is the IID that matches.
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

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = &self.data4;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data1, self.data2, self.data3, d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]
        )
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guid({self})")
    }
}
