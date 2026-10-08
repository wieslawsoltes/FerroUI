//! The headless platform.
//!
//! An application runs without a screen: its windows are sized and
//! activated like the windows of a desktop platform, input is simulated
//! through the extensions of a top-level ([`HeadlessWindowExtensions`]) and
//! frames are rendered when the render timer ticks, which a test can force
//! ([`FerroHeadlessPlatform::force_render_timer_tick`]).
//!
//! With the headless drawing (the default of
//! [`FerroHeadlessPlatformOptions`]) nothing is drawn: geometries behave
//! like their bounds and bitmaps have a size and no content. With another
//! rendering backend the frames of a window are kept, and
//! [`HeadlessWindowExtensions::get_last_rendered_frame`] returns the last
//! one.
//!
//! An application uses the platform with
//! [`FerroHeadlessPlatformExtensions::use_headless`] on its builder.

mod ferro_headless_platform;
mod headless_platform_render_interface;
mod headless_platform_stubs;
mod headless_render_timer;
mod headless_window_extensions;
mod headless_window_impl;
mod headless_window_surface;
mod i_headless_touch_pointer;
mod i_headless_window;

pub use ferro_headless_platform::{FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
pub use headless_window_extensions::HeadlessWindowExtensions;
pub use i_headless_touch_pointer::IHeadlessTouchPointer;

#[cfg(test)]
mod tests;
