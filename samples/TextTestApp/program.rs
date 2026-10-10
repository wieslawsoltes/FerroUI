//! Port of `Program.cs`: the desktop entry point of the TextTestApp sample.
//!
//! ```text
//! cargo run -p text-test-app
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p text-test-app
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which
//! ends the main loop; the process exits with the exit code the lifetime returns.
//!
//! Not ported:
//! - the statement that attaches `FontFeatureCollectionConverter` to the font feature
//!   collection type at run time, before the application is built: the framework has no
//!   run-time registration of a type converter for a type, and the collection type states its
//!   conversion from text itself (GAPS.md, T002);
//! - the developer tools of a debug build, which do not exist in the port (GAPS.md, T006).

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use text_test_app::App;

// Initialization code. Don't use any framework, third-party APIs or any code that relies on
// the dispatcher before the application is started: things aren't initialized yet and stuff
// might break.
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

// Application configuration, don't remove.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
}
