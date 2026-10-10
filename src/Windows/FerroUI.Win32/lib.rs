//! ferroui-win32
//!
//! The Windows platform backend: the windowing platform, the dispatcher
//! over the message loop, windows, screens, cursors, the clipboard, input
//! and the software surface, over the Win32 API.
//!
//! The design, the file table and the stages are in
//! `docs/porting/win32-platform.md`. What a stage has not built yet is
//! absent, or fails with a message that names its stage: nothing stands in
//! for it.
//!
//! The crate compiles on every host. What calls the system is compiled for
//! Windows only (`cfg(windows)`); the constants, the tables and the logic
//! that reads messages and computes styles are compiled, and tested,
//! everywhere.

// On a host that is not Windows the library holds the logic and no caller
// of it: the callers are the parts that call the system. The tests call it
// on every host.
#![cfg_attr(not(windows), allow(dead_code))]

pub mod input;
pub mod interop;

mod cursor_factory;
mod framebuffer_manager;
mod win32_dispatcher_impl;
mod platform_constants;
mod win32_platform_options;
mod win32_top_level_scene_info;
mod win32_type_extensions;
mod window_impl;
mod window_impl_app_wnd_proc;

#[cfg(windows)]
mod clipboard_impl;
#[cfg(windows)]
mod embedded_window_impl;
#[cfg(windows)]
mod offscreen_parent_window;
#[cfg(windows)]
mod popup_impl;
#[cfg(windows)]
mod screen_impl;
#[cfg(windows)]
mod simple_window;
#[cfg(windows)]
mod win32_platform;
#[cfg(windows)]
mod win32_platform_settings;
#[cfg(windows)]
mod win_screen;
#[cfg(windows)]
mod wnd_proc_guard;

pub use cursor_factory::cursor_resource_id;
pub use platform_constants::{PlatformConstants, Version};
pub use win32_platform_options::{Win32CompositionMode, Win32DpiAwareness, Win32PlatformOptions, Win32RenderingMode};
pub use win32_top_level_scene_info::Win32TopLevelSceneInfo;
pub use win32_type_extensions::Win32TypeExtensions;

#[cfg(windows)]
pub use clipboard_impl::ClipboardImpl;
#[cfg(windows)]
pub use cursor_factory::{CursorFactory, CursorImpl};
#[cfg(windows)]
pub use embedded_window_impl::EmbeddedWindowImpl;
#[cfg(windows)]
pub use framebuffer_manager::FramebufferManager;
#[cfg(windows)]
pub use popup_impl::PopupImpl;
#[cfg(windows)]
pub use screen_impl::ScreenImpl;
#[cfg(windows)]
pub use win32_dispatcher_impl::Win32DispatcherImpl;
#[cfg(windows)]
pub use win32_platform::{Win32ApplicationExtensions, Win32Platform};
#[cfg(windows)]
pub use win32_platform_settings::Win32PlatformSettings;
#[cfg(windows)]
pub use win_screen::WinScreen;
#[cfg(windows)]
pub use window_impl::WindowImpl;

/// Fails for a member of the backend that a later stage builds: the message
/// names the member and the stage of `docs/porting/win32-platform.md`.
#[cfg_attr(not(windows), allow(dead_code))]
#[track_caller]
pub(crate) fn not_built(member: &str, stage: u32) -> ! {
    panic!("{member} is not built yet: it belongs to stage {stage} of the Windows platform backend (docs/porting/win32-platform.md)")
}
