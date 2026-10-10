//! ferroui-wayland
//!
//! The windowing platform of FerroUI for Wayland compositors: windows over
//! `xdg_toplevel`, input, screens, cursors, and render surfaces over shared
//! memory buffers and over EGL. `docs/porting/wayland-platform.md` has the
//! design and the stages.
//!
//! A worker thread owns the connection to the compositor and renders; the UI
//! thread sends it commands and receives events as posted calls. The protocol
//! runs over `libwayland-client`, which is opened at run time together with
//! `libwayland-egl`, `libEGL` and `libxkbcommon`, so the crate builds without
//! them and an application that carries it starts on a system that has none.
//!
//! The part that talks to a compositor exists on Linux only, as in the
//! reference. What is plain logic (the key tables, the states of a configure,
//! the geometry of a window, the frames of pointer events) exists on every
//! system, which is where its tests run during development.

pub mod screens;
pub mod server;
pub mod ferro_wayland_exception;
pub mod wayland_platform_options;
pub mod xkb_key_transform;

#[cfg(target_os = "linux")]
pub mod wayland_cursor_factory;
#[cfg(target_os = "linux")]
pub mod wayland_glib_dispatcher;
#[cfg(target_os = "linux")]
pub mod wayland_platform;
pub mod ferro_wayland_platform_extensions;
#[cfg(target_os = "linux")]
pub mod wayland_surface_create_result;
#[cfg(target_os = "linux")]
pub mod wayland_top_level_factory;
#[cfg(target_os = "linux")]
pub mod window_impl;
#[cfg(target_os = "linux")]
pub mod window_impl_base;
#[cfg(target_os = "linux")]
mod window_impl_base_keyboard;
#[cfg(target_os = "linux")]
mod window_impl_base_pointer;
#[cfg(target_os = "linux")]
mod window_impl_sink;
#[cfg(target_os = "linux")]
pub mod xkb_common_keymap;
#[cfg(target_os = "linux")]
pub mod xkb_compose_state;
#[cfg(target_os = "linux")]
pub mod xkb_compose_table;
#[cfg(target_os = "linux")]
pub mod xkb_context;

pub use ferro_wayland_exception::{
    FerroWaylandException, FerroWaylandFlushException, FerroWaylandNetworkException, FerroWaylandPollException,
    FerroWaylandProtocolErrorException, FerroWaylandReadException,
};
pub use ferro_wayland_platform_extensions::FerroWaylandPlatformExtensions;
pub use wayland_platform_options::WaylandPlatformOptions;
