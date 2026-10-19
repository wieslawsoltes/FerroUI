//! `Guid`, the reference of a feature usage tracking session.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A 128-bit identifier (`System.Guid`), written as `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
// Deviation (DEVIATIONS.md, Splat): .NET's `Guid` is not part of the standard library; this one
// makes random version 4 identifiers from the random keys of the standard library's hasher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(u128);

impl Guid {
    /// The identifier whose bits are all zero (`Guid.Empty`).
    pub const EMPTY: Guid = Guid(0);

    /// A new random identifier (`Guid.NewGuid`).
    pub fn new_guid() -> Self {
        let count = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |time| time.as_nanos());
        let random = |salt: u64| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(salt);
            hasher.write_u64(count);
            hasher.write_u128(nanos);
            hasher.finish()
        };
        let mut value = (u128::from(random(0x5EED_0001)) << 64) | u128::from(random(0x5EED_0002));
        // Version 4, variant 1 (RFC 4122).
        value = (value & !(0xF_u128 << 76)) | (0x4_u128 << 76);
        value = (value & !(0x3_u128 << 62)) | (0x2_u128 << 62);
        Guid(value)
    }

    /// The identifier of the 128 bits `value`.
    pub const fn from_u128(value: u128) -> Self {
        Guid(value)
    }

    /// The 128 bits of the identifier.
    pub const fn as_u128(&self) -> u128 {
        self.0
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (v >> 96) as u32,
            (v >> 80) as u16,
            (v >> 64) as u16,
            (v >> 48) as u16,
            v & 0xFFFF_FFFF_FFFF
        )
    }
}
