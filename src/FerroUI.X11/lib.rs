//! ferroui-x11
//!
//! The windowing platform of FerroUI for the X Window System: windows,
//! input, screens, cursors, the clipboard and a software render surface
//! over a connection to an X server. `docs/porting/x11-platform.md` has
//! the design and the stages.
//!
//! The platform talks to the server through Xlib and its extension
//! libraries, which are opened at run time, so the crate builds without
//! them. It builds on every Unix system (the libraries exist wherever an X
//! server does) and is empty elsewhere; the desktop entry point selects it
//! on Linux.
//!
//! Two modules have no dependency on the server at all, and exist on every
//! system: the key symbols and the key tables.

pub mod keysyms;
pub mod x11_key_transform;

#[cfg(unix)]
pub mod activity_tracking_helper;
#[cfg(unix)]
pub mod dispatching;
#[cfg(unix)]
pub use ferroui_freedesktop::event;
#[cfg(unix)]
pub mod glx;
#[cfg(unix)]
pub mod lib_c;
#[cfg(unix)]
pub(crate) mod pixel_buffer;
#[cfg(unix)]
pub mod raw_event_grouping;
#[cfg(unix)]
pub mod screens;
#[cfg(unix)]
pub mod selections;
#[cfg(unix)]
pub mod transparency_helper;
#[cfg(unix)]
pub mod x11_active_window_tracker;
#[cfg(unix)]
pub mod x11_atoms;
#[cfg(unix)]
pub mod x11_cursor_factory;
#[cfg(unix)]
pub mod x11_deferred_display_dispatcher;
#[cfg(unix)]
pub mod x11_egl_helper;
#[cfg(unix)]
pub mod x11_enum_extensions;
#[cfg(unix)]
pub mod x11_enums;
#[cfg(unix)]
pub mod x11_exception;
#[cfg(unix)]
pub mod x11_focus_proxy;
#[cfg(unix)]
pub mod x11_framebuffer_surface;
#[cfg(unix)]
pub mod x11_globals;
#[cfg(unix)]
pub mod x11_icon_loader;
#[cfg(unix)]
pub mod x11_info;
#[cfg(unix)]
pub mod x11_platform;
#[cfg(unix)]
pub mod x11_structs;
#[cfg(unix)]
pub mod x11_window;
#[cfg(unix)]
pub(crate) mod x11_window_ime;
#[cfg(unix)]
pub(crate) mod x11_window_xim;
#[cfg(unix)]
pub mod x11_window_info;
#[cfg(unix)]
pub mod x11_window_modes;
#[cfg(unix)]
pub mod x_error;
#[cfg(unix)]
pub mod x_resources;
#[cfg(unix)]
pub mod x_shm;
#[cfg(unix)]
pub mod xi2_manager;
#[cfg(unix)]
pub mod xi_structs;
#[cfg(unix)]
pub mod xlib;

#[cfg(unix)]
pub use x11_platform::{
    initialize_x11_platform, FerroX11Platform, FerroX11PlatformExtensions, X11PlatformOptions, X11RenderingMode,
};
