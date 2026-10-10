//! The Windows.UI.Composition mode: a window is presented through a
//! surface of the composition tree of the Windows Runtime, which also has
//! the blur effects behind a window (acrylic, mica).
//!
//! What is logic (the effects as values, the versions of Windows, which
//! visual an effect shows) compiles and is tested on every host; what
//! calls the compositor is Windows only.

mod d2d_effects;
mod win_ui_composited_window;
mod win_ui_composited_window_surface;
mod win_ui_composition_shared;
#[cfg(windows)]
mod win_ui_composition_utils;
mod win_ui_compositor_connection;
mod win_ui_effect_base;

#[cfg(windows)]
pub(crate) use win_ui_compositor_connection::WinUiCompositorConnection;

#[cfg(all(test, windows))]
mod win_ui_composition_tests;

/// The object of a call that succeeded: the proxies of the generated
/// bindings return `None` for a null pointer, which the proxies of the
/// reference hand on as a null object that fails at its first use.
#[cfg(windows)]
pub(crate) trait Required<T> {
    fn required(self) -> Result<T, ferroui_microcom::HResult>;
}

#[cfg(windows)]
impl<T> Required<T> for Result<Option<T>, ferroui_microcom::HResult> {
    fn required(self) -> Result<T, ferroui_microcom::HResult> {
        self?.ok_or(ferroui_microcom::HResult::POINTER)
    }
}
