//! What the worker needs of the system (the directory `Server/Interop` of
//! the reference): the connection, the wake-up descriptor and the calls of
//! the C library.
//!
//! `WaylandEglNativeMethods.cs` and `XkbCommonNativeMethods.cs` are not
//! ported: their declarations are those of the crates `wayland-egl` and
//! `xkbcommon-dl`.

pub mod unsafe_native_methods;
pub mod wakeup_fd;
pub mod wayland_connection;
