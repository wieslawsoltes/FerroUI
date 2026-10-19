//! Port of `Program.cs` of the desktop head of the example: the desktop entry point.
//!
//! ```text
//! cargo run -p reactiveui-example
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p reactiveui-example
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which ends
//! the main loop; the session of the example is saved as on any exit.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use reactiveui_example::ExampleAppBuilderExtensions;

/// Builds the application with the shared showcase setup and the desktop platform backend
/// (the `Build...App` method of the reference).
fn build_ferro_app() -> AppBuilder {
    ExampleAppBuilderExtensions::create()
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
}

/// Runs the example app (`Main`).
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
