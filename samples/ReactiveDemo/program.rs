//! The desktop entry point of the sample (the reference, a WPF application, starts from
//! `App.xaml`).
//!
//! ```text
//! cargo run -p reactive-demo
//! cargo run -p reactive-demo --no-default-features
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p reactive-demo
//! ```
//!
//! With the default feature `online` the search queries crates.io; without it the search runs
//! over an in-memory catalogue of crates.
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which ends
//! the main loop; the process exits with the exit code the lifetime returns.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_reactiveui::ReactiveUIBuilder;
use ferroui_reactiveui_controls::AppBuilderExtensions;
use reactive_demo::App;

/// App configuration, used by the entry point.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        // ReactiveUI with the services of FerroUI: the scheduler of the user-interface thread as
        // the main thread scheduler, the activation of the views, the observation of the
        // properties of controls, the command binding of controls and the data-template hook of
        // the list. The views are registered by the application (`App::initialize`), as the
        // reference's `App` does.
        .use_reactive_ui(|_: &ReactiveUIBuilder| {})
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
