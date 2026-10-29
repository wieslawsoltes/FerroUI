//! The desktop entry point of the Material theme gallery: the gallery in a window of the platform
//! (macOS, Linux, Windows), and its capture mode.
//!
//! ```text
//! cargo run -p material-theme-gallery --bin material-theme-gallery
//! FERROUI_MATERIAL_GALLERY_CAPTURE=target/material-theme-gallery cargo run -p material-theme-gallery --bin material-theme-gallery
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p material-theme-gallery --bin material-theme-gallery
//! FERROUI_MATERIAL_GALLERY_START_PAGE=timepickers FERROUI_MATERIAL_GALLERY_START_VARIANT=Dark cargo run -p material-theme-gallery --bin material-theme-gallery
//! ```
//!
//! With `FERROUI_MATERIAL_GALLERY_CAPTURE=<directory>` the gallery writes a picture of every page in
//! every setting to the directory and exits: with 0 when every picture was written without a
//! problem, with 1 otherwise (`capture.rs` lists the options). With `FERROUI_SMOKE_EXIT_MS=<n>`
//! the main window is closed after `n` milliseconds, and with `FERROUI_SMOKE_SCREENSHOT=<file>`
//! a picture of the main window is written (`sample_support::smoke_run`).

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_themes_material::MaterialAppBuilderExtension;
use std::path::PathBuf;
use material_theme_gallery::{capture, App};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    if capture::failed() && exit_code == 0 {
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::from(exit_code.clamp(0, 255) as u8)
}

fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .with_material_fonts()
        .after_setup(|_| {
            match std::env::var_os("FERROUI_MATERIAL_GALLERY_CAPTURE").filter(|directory| !directory.is_empty()) {
                Some(directory) => capture::start(PathBuf::from(directory)),
                None => sample_support::smoke_run(),
            }
        })
        .log_to_trace(LogEventLevel::Warning, &[])
}
