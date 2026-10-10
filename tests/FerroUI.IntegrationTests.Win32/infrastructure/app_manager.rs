//! Sets the application up once for all tests.
//!
//! Upstream starts a UI thread that sets the application up and runs the
//! main loop, and its test executor runs every test on the dispatcher of
//! that thread. Here the thread of the test binary is the UI thread: the
//! application is set up on it, the tests run on it one after the other,
//! and a test runs the message loop where upstream's awaits.

use ferroui_base::threading::Dispatcher;
use ferroui_controls::{AppBuilder, Application};
use ferroui_themes_fluent::FluentTheme;

#[cfg(windows)]
fn use_platform(builder: AppBuilder) -> AppBuilder {
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use ferroui_win32::Win32ApplicationExtensions;

    builder.use_win32().use_skia().use_harfbuzz()
}

#[cfg(not(windows))]
fn use_platform(_builder: AppBuilder) -> AppBuilder {
    panic!("the integration tests of the Windows platform backend run on Windows only")
}

/// Sets the application up on the calling thread, which becomes the UI
/// thread of the tests.
pub fn ensure_app_initialized() {
    let app_builder = use_platform(AppBuilder::configure::<Application>()).setup_without_starting();

    app_builder.instance().expect("the application").styles().add(FluentTheme::new().as_style());

    // Ensure that the dispatcher of the UI thread is the one of this thread
    Dispatcher::ui_thread().verify_access();
}
