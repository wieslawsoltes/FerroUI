//! `System.Guid` as the domain of the tests uses it: an identifier that is unique per instance.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_GUID: AtomicU64 = AtomicU64::new(1);

/// A unique identifier (`Guid`).
///
/// Deviation (DEVIATIONS.md, DynamicData): `Guid.NewGuid()` is a number from a counter of the
/// process, not a random 128-bit value; the tests only need identifiers that differ. It formats as
/// a GUID does (`D` with `Display`, `B` with [`to_string_b`](Self::to_string_b)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(u128);

impl Guid {
    /// The empty identifier (`Guid.Empty`).
    pub const EMPTY: Guid = Guid(0);

    /// A new identifier, different from every other one of the process (`Guid.NewGuid()`).
    pub fn new_guid() -> Self {
        Guid(u128::from(NEXT_GUID.fetch_add(1, Ordering::Relaxed)))
    }

    /// The identifier in braces (`ToString("B")`).
    pub fn to_string_b(&self) -> String {
        format!("{{{self}}}")
    }
}

impl fmt::Display for Guid {
    /// The form `D` of a GUID: `00000000-0000-0000-0000-000000000000`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            (v >> 96) as u32,
            (v >> 80) as u16,
            (v >> 64) as u16,
            (v >> 48) as u16,
            v & 0xffff_ffff_ffff
        )
    }
}
