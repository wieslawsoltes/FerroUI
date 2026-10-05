//! Whether a [`DateTime`](super::DateTime) is local time, UTC or neither
//! (.NET `System.DateTimeKind`).

/// The kind of time a [`DateTime`](super::DateTime) represents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DateTimeKind {
    /// Neither local time nor UTC is stated.
    #[default]
    Unspecified = 0,
    /// Coordinated Universal Time.
    Utc = 1,
    /// Local time.
    Local = 2,
}
