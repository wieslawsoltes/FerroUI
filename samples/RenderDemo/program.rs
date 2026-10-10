//! Port of the entry point of `App.xaml.cs` (`Main` and the application builder of the
//! managed original): the desktop entry point of the RenderDemo sample.
//!
//! ```text
//! cargo run -p render-demo
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p render-demo
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which
//! ends the main loop; the process exits with the exit code the lifetime returns.
//!
//! Not ported, because the platform and the tools they name do not exist in the port: the
//! options of the Windows platform (`OverlayPopups = true`) and the developer tools of a debug
//! build.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use render_demo::App;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// App configuration, used by the entry point.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
}
