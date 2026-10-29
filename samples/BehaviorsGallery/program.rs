//! Port of `Program.cs`: the desktop entry point of the behaviors gallery.
//!
//! ```text
//! cargo run -p behaviors-gallery
//! FERROUI_SMOKE_PAGES=all cargo run -p behaviors-gallery
//! FERROUI_SMOKE_PAGES=ported FERROUI_SMOKE_PAGE_MS=500 cargo run -p behaviors-gallery
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p behaviors-gallery
//! ```
//!
//! With `FERROUI_SMOKE_PAGES` the gallery shows its pages one after the other and exits
//! (`smoke.rs`): with 1 when a page failed to be created, with the exit code of the lifetime
//! otherwise. With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds,
//! and with `FERROUI_SMOKE_SCREENSHOT=<file>` a picture of the main window is written
//! (`sample_support::smoke_run`).

use behaviors_gallery::{smoke, App};
use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_reactiveui_controls::AppBuilderExtensions;

/// The application builder of the reference: the application with the Inter font, the platform and
/// ReactiveUI.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .with_inter_font()
        .use_platform_detect()
        .use_reactive_ui(|_| {})
        .after_setup(|_| {
            smoke::start();
            sample_support::smoke_run();
        })
        .log_to_trace(LogEventLevel::Warning, &[])
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    if smoke::failed() {
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::from(exit_code.clamp(0, 255) as u8)
}
