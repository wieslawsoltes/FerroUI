//! ferroui-wayland
//!
//! The windowing platform of FerroUI for Wayland compositors: windows over
//! `xdg_toplevel`, popups, input, screens, cursors, the clipboard and drag
//! and drop over `wl_data_device`, input methods over `zwp_text_input_v3`,
//! the parent handle of a portal dialog over `xdg-foreign`, and optional desktop layers over
//! `zwlr_layer_shell_v1`. Render surfaces support shared memory, EGL and native Vulkan WSI. `docs/porting/wayland-platform.md` has the
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

pub mod clipboard;
pub mod screens;
pub mod server;
pub mod ferro_wayland_exception;
pub mod text_input_options_converter;
pub mod wayland_platform_options;
pub mod wayland_text_utils;
pub mod xkb_key_transform;

#[cfg(target_os = "linux")]
pub mod i_wayland_xdg_top_level_export;
#[cfg(target_os = "linux")]
pub mod layer_shell;
#[cfg(target_os = "linux")]
pub mod popup_impl;
#[cfg(target_os = "linux")]
mod popup_impl_sink;
#[cfg(target_os = "linux")]
pub mod wayland_conversion_extensions;
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
mod window_impl_base_drag_drop;
#[cfg(target_os = "linux")]
mod window_impl_base_keyboard;
#[cfg(target_os = "linux")]
mod window_impl_base_pointer;
#[cfg(target_os = "linux")]
mod window_impl_sink;
#[cfg(target_os = "linux")]
mod window_impl_text_input;
#[cfg(target_os = "linux")]
pub mod xkb_common_keymap;
#[cfg(target_os = "linux")]
pub mod xkb_compose_state;
#[cfg(target_os = "linux")]
pub mod xkb_compose_table;
#[cfg(target_os = "linux")]
pub mod xkb_context;

pub use ferro_wayland_exception::{
    FerroWaylandError, FerroWaylandFlushException, FerroWaylandNetworkException, FerroWaylandPollException,
    FerroWaylandProtocolErrorException, FerroWaylandReadException,
};
#[allow(deprecated)]
pub use ferro_wayland_exception::FerroWaylandException;
pub use ferro_wayland_platform_extensions::FerroWaylandPlatformExtensions;
pub use wayland_platform_options::{WaylandPlatformOptions, WaylandRenderingMode};
