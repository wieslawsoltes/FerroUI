//! Port of `Program.cs`: the desktop entry point of the AppWithoutLifetime sample. The
//! application is set up without an application lifetime; the entry point creates the main
//! window and runs the main loop until that window is closed.
//!
//! ```text
//! cargo run -p app-without-lifetime
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p app-without-lifetime
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` (not a port: the smoke option of the desktop entry points
//! of the samples) the main window is closed after `n` milliseconds, which ends the main loop.
//! There is no lifetime whose main window `sample_support::smoke_run` could close, so the
//! entry point closes the window it created.
//!
//! Not ported, because the tools it names do not exist in the port: the developer tools of a
//! debug build (GAPS.md, A001).

use app_without_lifetime::{App, MainWindow};
use ferroui_base::logging::LogEventLevel;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::Ref;
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::time::Duration;

// Initialization code. Don't use any framework, third-party APIs or any code that relies on
// the dispatcher before app_main is called: things aren't initialized yet and stuff might
// break.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    build_ferro_app().start(app_main, &args);
}

fn app_main(app: &Ref<Application>, _args: &[String]) {
    let main_window = MainWindow::new();
    smoke_run(&main_window);
    app.run_window(main_window);
}

// Application configuration, don't remove.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>().use_platform_detect().log_to_trace(LogEventLevel::Warning, &[])
}

/// The smoke run of the entry point: with `FERROUI_SMOKE_EXIT_MS=<n>` the window is closed
/// after `n` milliseconds.
fn smoke_run(main_window: &Ref<MainWindow>) {
    if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) {
        println!("Will close the main window after {ms} ms");
        let main_window = main_window.clone();
        // The timer stops itself after its only tick.
        let _timer = DispatcherTimer::run_once(
            move || {
                println!("Timer fired: closing the main window");
                main_window.close();
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }
}
