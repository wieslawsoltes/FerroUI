//! `System.Guid` as the domain of the tests uses it: `uuid::Uuid`, the identifier type of the
//! workspace, with identifiers that are unique per process.

use std::sync::atomic::{AtomicU64, Ordering};

pub use uuid::Uuid as Guid;

static NEXT_GUID: AtomicU64 = AtomicU64::new(1);

/// A new identifier, different from every other one of the process (`Guid.NewGuid()`). It formats
/// as a GUID does: `D` with `Display`, `B` with `braced()`.
///
/// Deviation (DEVIATIONS.md, DynamicData): `Guid.NewGuid()` is a number from a counter of the
/// process, not a random 128-bit value; the tests only need identifiers that differ.
pub fn new_guid() -> Guid {
    Guid::from_u128(u128::from(NEXT_GUID.fetch_add(1, Ordering::Relaxed)))
}
