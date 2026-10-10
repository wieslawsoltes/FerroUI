//! Port of `Program.cs`: the desktop entry point of the Sandbox sample.
//!
//! ```text
//! cargo run -p sandbox
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p sandbox
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which
//! ends the main loop; the process exits with the exit code the lifetime returns.
//!
//! Not ported, because the tools it names do not exist in the port: the developer tools of a
//! debug build (GAPS.md, S001).

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use sandbox::App;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
}
