//! The gauntlet in a desktop window, as the desktop harness of the Cranpose benchmark runs it.
//!
//! ```text
//! PERF_TIER=16 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
//! PERF_SCENARIO=wall PERF_TIER=20 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
//! ```
//!
//! `PERF_TIER` (default 5), `PERF_FREEZE` (default 0: never) and `PERF_SCENARIO` (`wall` for the
//! still wall, anything else for the gauntlet) choose what runs; `PERF_FONTS` names the folder of
//! the Roboto files (default `fonts`). The `PERF` lines the harness reads are written to standard
//! output. With the feature `vello`, `FERROUI_RENDERER=vello` (or `vello-hybrid`, `vello-gpu`,
//! `vello-cpu`) renders with the Vello backend; with the feature `progpu`,
//! `FERROUI_RENDERER=progpu` renders with the ProGPU backend and lays text out with its shaper.

use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use gauntlet_benchmark::{App, Launch};

fn variable(name: &str, fallback: i32) -> i32 {
    std::env::var(name).ok().and_then(|value| value.trim().parse().ok()).unwrap_or(fallback)
}

fn main() -> std::process::ExitCode {
    #[cfg(all(windows, feature = "renderer-measurement"))]
    gauntlet_benchmark::measurement::start();
    Launch::set(Launch {
        tier: variable("PERF_TIER", 5),
        freeze: variable("PERF_FREEZE", 0),
        wall: std::env::var("PERF_SCENARIO").is_ok_and(|scenario| scenario == "wall"),
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(all(windows, feature = "renderer-measurement"))]
    let builder = gauntlet_benchmark::measurement::use_measured_renderer(builder);
    #[cfg(all(target_os = "macos", feature = "quartz"))]
    let builder = builder.with(std::rc::Rc::new(ferroui_native::FerroNativePlatformOptions {
        rendering_mode: vec![if cfg!(feature = "quartz-metal") {
            ferroui_native::FerroNativeRenderingMode::Metal
        } else {
            ferroui_native::FerroNativeRenderingMode::Software
        }],
        ..Default::default()
    }));
    let builder = builder.use_platform_detect();
    #[cfg(feature = "vello")]
    let builder = use_renderer_of_the_environment(builder);
    #[cfg(feature = "progpu")]
    let builder = use_progpu_of_the_environment(builder);
    let exit_code = builder.start_with_classic_desktop_lifetime(&args);
    #[cfg(feature = "vello")]
    ferroui_vello::perf::print_summary();
    std::process::ExitCode::from(exit_code as u8)
}

/// The render backend asked for with `FERROUI_RENDERER`: `vello` (the modes of the Vello backend
/// in their default order), `vello-hybrid`, `vello-gpu` or `vello-cpu`. Anything else, and no
/// value, leaves the backend of the platform.
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
    builder.with(std::rc::Rc::new(options)).use_vello()
}

/// The ProGPU backend when `FERROUI_RENDERER` is `progpu`: its render interface, its font manager
/// and its text shaper in place of the ones of the platform. Anything else, and no value, leaves
/// the builder as it is.
#[cfg(feature = "progpu")]
fn use_progpu_of_the_environment(builder: AppBuilder) -> AppBuilder {
    use ferroui_progpu::ProGpuApplicationExtensions;
    match std::env::var("FERROUI_RENDERER").ok().as_deref() {
        Some("progpu") => builder.use_progpu(),
        _ => builder,
    }
}
