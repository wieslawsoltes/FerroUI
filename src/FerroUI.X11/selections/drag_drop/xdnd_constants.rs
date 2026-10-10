//! The versions of the protocol (the port of `XdndConstants.cs`).

// Spec: every application that supports XDND version N must also support all previous versions (3 to N-1).
pub const MIN_XDND_VERSION: u8 = 3;
pub const XDND_VERSION: u8 = 5;
