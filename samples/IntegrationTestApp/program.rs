//! Port of `Program.cs`: the desktop entry point of the IntegrationTestApp sample.
//!
//! ```text
//! cargo run -p integration-test-app
//! cargo run -p integration-test-app -- --overlayPopups
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p integration-test-app
//! ```
//!
//! With `--overlayPopups` the popups of the application are shown inside its windows. With
//! `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which ends
//! the main loop; the process exits with the exit code the lifetime returns.
//!
//! Not ported, because the platform and the tools they name do not exist in the port: the
//! options of the Windows platform (`OverlayPopups`), the native text box of that platform
//! (`Win32TextBoxFactory`) and the developer tools of a debug build.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use integration_test_app::App;

// Initialization code. Don't use any framework, third-party APIs or any
// SynchronizationContext-reliant code before AppMain is called: things aren't initialized
// yet and stuff might break.
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    integration_test_app::set_overlay_popups(args.iter().any(|arg| arg == "--overlayPopups"));

    let builder = build_ferro_app();
    #[cfg(target_os = "macos")]
    let builder = builder.with(std::rc::Rc::new(ferroui_native::FerroNativePlatformOptions {
        overlay_popups: integration_test_app::overlay_popups(),
        ..Default::default()
    }));
    let exit_code = builder.start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// App configuration, used by the entry point.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| {
            #[cfg(target_os = "macos")]
            integration_test_app::embedding::NativeTextBox::set_factory(Some(std::rc::Rc::new(
                integration_test_app::embedding::MacOSTextBoxFactory,
            )));
            sample_support::smoke_run();
        })
        .log_to_trace(LogEventLevel::Warning, &[])
}
