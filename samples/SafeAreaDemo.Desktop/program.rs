//! Port of `Program.cs` of `SafeAreaDemo.Desktop`: the desktop entry point of the SafeAreaDemo
//! sample. The main window shows the main view of the sample (`samples/SafeAreaDemo`); a
//! desktop window has neither an insets manager nor an input pane, so the view shows its
//! bounds, a safe area equal to them and the options, which change nothing there but the theme
//! variant.
//!
//! ```text
//! cargo run -p safe-area-demo-desktop
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p safe-area-demo-desktop
//! FERROUI_SMOKE_STEPS_MS=800 FERROUI_SMOKE_SCREENSHOTS=/path/dir cargo run -p safe-area-demo-desktop
//! ```
//!
//! Additions of the port, for the smoke runs:
//!
//! - `FERROUI_SMOKE_EXIT_MS=<n>`: the main window is closed after `n` milliseconds, which ends
//!   the main loop; with `FERROUI_SMOKE_SCREENSHOT=<file.png>` a picture of the main window,
//!   drawn by the framework, is written first (`sample_support::smoke_run`).
//! - `FERROUI_SMOKE_STEPS_MS=<n>`: the options of the view are changed one after the other, one
//!   every `n` milliseconds, and the state of the view model is printed after each
//!   (`safe_area_demo::smoke`); with `FERROUI_SMOKE_SCREENSHOTS=<directory>` a picture of the
//!   main window is written after each change (`step-<index>-<name>.png`, the frame of the
//!   framework, not a capture of the screen). The main window is closed after the last change.
//!
//! The process exits with the exit code the lifetime returns.
//!
//! The project of the reference also references the VNC headless platform, the Linux
//! framebuffer platform and the X11 platform, which `Program.cs` does not use: the platform
//! detection of the desktop package (`use_platform_detect`) chooses the X11 platform on Linux.
//! Not ported, because the tools it names do not exist in the port: the developer tools of a
//! debug build (`#if DEBUG .WithDeveloperTools()`; `samples/SafeAreaDemo/GAPS.md`, SA001).

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use safe_area_demo::view_models::MainViewModel;
use safe_area_demo::views::MainView;
use safe_area_demo::{smoke, App};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = build_ferro_app().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// The method of the reference that makes the application builder.
fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| {
            sample_support::smoke_run();
            steps_run();
        })
        .log_to_trace(LogEventLevel::Warning, &[])
}

fn main_window() -> Option<ferroui_base::Ref<Window>> {
    let lifetime = Application::current()?.application_lifetime()?;
    lifetime.as_classic_desktop_style_application_lifetime()?.main_window()
}

/// The view model of the main view of the main window, once the view is loaded.
fn main_view_model() -> Option<Rc<MainViewModel>> {
    let view = main_window()?.content()?;
    let view = ferroui_base::data::core::ValueTypes::as_object(&*view)?.cast::<MainView>()?;
    smoke::main_view_model(&view)
}

/// The run asked for with `FERROUI_SMOKE_STEPS_MS` and `FERROUI_SMOKE_SCREENSHOTS`.
fn steps_run() {
    let Some(step_ms) = std::env::var("FERROUI_SMOKE_STEPS_MS").ok().and_then(|value| value.parse::<u64>().ok()) else {
        return;
    };
    let directory = std::env::var_os("FERROUI_SMOKE_SCREENSHOTS").filter(|directory| !directory.is_empty()).map(PathBuf::from);
    let shown_directory = directory.clone();
    smoke::run(
        main_view_model,
        Duration::from_millis(step_ms),
        |line| println!("{line}"),
        move |index, name| {
            if let Some(directory) = &directory {
                let path = directory.join(format!("step-{index:02}-{}.png", name.replace('=', "-")));
                match sample_support::save_main_window_frame(&path) {
                    Ok(size) => println!("Screenshot: {} ({} by {})", path.display(), size.width, size.height),
                    Err(error) => println!("Screenshot: {} could not be written: {error}", path.display()),
                }
            }
        },
        || {
            println!("Closing the main window");
            if let Some(window) = main_window() {
                window.close();
            }
        },
    );
    if let Some(directory) = shown_directory {
        // The view as it shows first, before any change.
        let _timer = ferroui_base::threading::DispatcherTimer::run_once(
            move || {
                let path = directory.join("shown.png");
                match sample_support::save_main_window_frame(&path) {
                    Ok(size) => println!("Screenshot: {} ({} by {})", path.display(), size.width, size.height),
                    Err(error) => println!("Screenshot: {} could not be written: {error}", path.display()),
                }
            },
            Duration::from_millis(step_ms / 2),
            ferroui_base::threading::DispatcherPriority::NORMAL,
        )
        .detach();
    }
}
