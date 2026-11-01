//! `Guid`, the reference of a feature usage tracking session.

use std::fmt;
use uuid::Uuid;

/// A 128-bit identifier (`System.Guid`), written as `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
///
/// It is a [`Uuid`] of the `uuid` crate, the identifier type of the workspace
/// (docs/idiomatic-rust.md): the two convert into each other with `From`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(Uuid);

impl Guid {
    /// The identifier whose bits are all zero (`Guid.Empty`).
    pub const EMPTY: Guid = Guid(Uuid::nil());

    /// A new random identifier of version 4 (`Guid.NewGuid`), from the random generator of the
    /// system.
    pub fn new_guid() -> Self {
        Guid(Uuid::new_v4())
    }

    /// The identifier of the 128 bits `value`.
    pub const fn from_u128(value: u128) -> Self {
        Guid(Uuid::from_u128(value))
    }

    /// The 128 bits of the identifier.
    pub const fn as_u128(&self) -> u128 {
        self.0.as_u128()
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

impl fmt::Display for Guid {
    /// The form `D` of `Guid.ToString()`: hyphenated, in lower case.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.hyphenated(), f)
    }
}
