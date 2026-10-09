//! Port of `Program.cs` of `ControlCatalog.Desktop`: the desktop entry
//! point of the ControlCatalog sample.
//!
//! ```text
//! cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=20000 FERROUI_SMOKE_PAGES=150 cargo run -p control-catalog-desktop
//! FERROUI_CATALOG_THEME=simple cargo run -p control-catalog-desktop
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n`
//! milliseconds, which ends the main loop; the process exits with the exit
//! code the lifetime returns. With `FERROUI_SMOKE_PAGES=<n>` the pages of
//! the catalog are shown one after the other, `n` milliseconds each (the
//! role of the `--full-headless` run of the managed original, which
//! selects every page).
//!
//! Not ported, because the platforms and options they name do not exist
//! yet: the command line switches `--wait-for-attach`, `--fbdev`, `--vnc`,
//! `--full-headless`, `--drm`, `--dxgi` (with `--scaling`,
//! `--orientation`, `--card`), and of the application builder the data
//! annotations validation, the Wayland, X11, Vulkan and composition
//! options, the Inter font, the developer tools and the native control
//! samples of `NativeControls/` (`EmbedSample.Implementation`).

use control_catalog::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::time::Duration;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let builder = build_ferro_app();

    let exit_code = builder.start_with_classic_desktop_lifetime(&args);
    // With `FERROUI_VELLO_PERF` the Vello backend says where the time of its
    // frames went.
    #[cfg(feature = "vello")]
    ferroui_vello::perf::print_summary();
    std::process::ExitCode::from(exit_code as u8)
}

/// The application builder of the catalog.
pub fn build_ferro_app() -> AppBuilder {
    let builder = AppBuilder::configure::<App>().use_platform_detect();
    #[cfg(feature = "vello")]
    let builder = use_renderer_of_the_environment(builder);
    builder.after_setup(|_| smoke_run()).log_to_trace(LogEventLevel::Warning, &[])
}

/// The render backend asked for with `FERROUI_RENDERER`: `vello` (the modes
/// of the Vello backend in their default order), `vello-hybrid`, `vello-gpu`
/// or `vello-cpu`. Anything else, and no value, leaves the backend of the
/// platform.
#[cfg(feature = "vello")]
fn use_renderer_of_the_environment(builder: AppBuilder) -> AppBuilder {
    use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
    let options = match std::env::var("FERROUI_RENDERER").ok().as_deref() {
        Some("vello") => VelloOptions::default(),
        Some("vello-hybrid") => VelloOptions::with_rendering_mode(VelloRenderingMode::Hybrid),
        Some("vello-gpu") => VelloOptions::with_rendering_mode(VelloRenderingMode::Gpu),
        Some("vello-cpu") => VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu),
        _ => return builder,
    };
    eprintln!("ControlCatalog: the Vello backend, modes {:?}", options.rendering_mode_order());
    builder.with(std::rc::Rc::new(options)).use_vello()
}

fn environment_milliseconds(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|value| value.parse::<u64>().ok())
}

/// The smoke run asked for with `FERROUI_SMOKE_EXIT_MS` and
/// `FERROUI_SMOKE_PAGES`.
fn smoke_run() {
    if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_PAGES") {
        control_catalog::show_every_page(Duration::from_millis(ms));
    }

    if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_EXIT_MS") {
        println!("Will close the main window after {ms} ms");
        // The timer stops itself after its only tick.
        let _timer = DispatcherTimer::run_once(
            || {
                println!("Timer fired: closing the main window");
                let lifetime = Application::current().and_then(|application| application.application_lifetime());
                let main_window =
                    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|l| l.main_window());
                if let Some(main_window) = main_window {
                    main_window.close();
                }
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }
}
