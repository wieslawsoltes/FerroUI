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

    rendering_options(builder).use_win32().use_skia().use_harfbuzz()
}

/// Upstream's tests run with the default options. For the tests of what a
/// window presents the run can be given one rendering mode without a
/// fallback: `FERROUI_SMOKE_RENDERING=software` or `angle`, the variable
/// the smoke runs of the examples read.
#[cfg(windows)]
fn rendering_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_win32::{Win32PlatformOptions, Win32RenderingMode};
    use std::rc::Rc;

    let mut options = Win32PlatformOptions::default();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("software") => options.rendering_mode = vec![Win32RenderingMode::Software],
        Some("angle") => {
            options.rendering_mode = vec![Win32RenderingMode::AngleEgl];
            options.composition_mode = vec![smoke_composition_mode()];
        }
        Some(other) => panic!("FERROUI_SMOKE_RENDERING: unknown mode {other:?} (software, angle)"),
        None => return builder,
    }
    println!("rendering modes of the run: {:?}; composition modes: {:?}", options.rendering_mode, options.composition_mode);
    builder.with(Rc::new(options))
}

/// The composition mode of a smoke run through ANGLE, without a fallback:
/// `FERROUI_SMOKE_COMPOSITION=redirection|dcomp|winui|dxgi`; the redirection surface
/// of the window when the variable is not set.
#[cfg(windows)]
fn smoke_composition_mode() -> ferroui_win32::Win32CompositionMode {
    use ferroui_win32::Win32CompositionMode;

    match std::env::var("FERROUI_SMOKE_COMPOSITION").ok().as_deref() {
        None | Some("redirection") => Win32CompositionMode::RedirectionSurface,
        Some("dcomp") => Win32CompositionMode::DirectComposition,
        Some("winui") => Win32CompositionMode::WinUIComposition,
        Some("dxgi") => Win32CompositionMode::LowLatencyDxgiSwapChain,
        Some(other) => panic!("FERROUI_SMOKE_COMPOSITION: unknown mode {other:?} (redirection, dcomp, winui, dxgi)"),
    }
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
