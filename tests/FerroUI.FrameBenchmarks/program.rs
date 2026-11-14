//! A scene of the frame benchmarks in a desktop window, for the window suite of the local
//! performance gate (the GPU backends and the start of a real process).
//!
//! ```text
//! FRAMES_SCENE=text PERF_REPORT=1 cargo run --release -p ferroui-frame-benchmarks
//! ```
//!
//! `FRAMES_SCENE` names the scene (`scenes.rs`; default `text`); `PERF_FONTS` a folder of the
//! Roboto files to use instead of those in the program. It prints `PERF scene <name>` when the
//! scene is built, `PERF first_frame` on the first animation frame and, with `PERF_REPORT=1`,
//! `PERF fps N` once a second. The application is the library's (`application.rs`); the hosts of
//! the devices run the same one.

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn main() -> std::process::ExitCode {
    use ferroui_controls::AppBuilder;
    use ferroui_desktop::AppBuilderDesktopExtensions;
    use ferroui_frame_benchmarks::App;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(all(target_os = "macos", feature = "quartz"))]
    let builder = builder.with(std::rc::Rc::new(ferroui_native::FerroNativePlatformOptions {
        rendering_mode: vec![if cfg!(feature = "quartz-metal") {
            ferroui_native::FerroNativeRenderingMode::Metal
        } else {
            ferroui_native::FerroNativeRenderingMode::Software
        }],
        ..Default::default()
    }));
    let exit_code = builder.use_platform_detect().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// On a device the scene runs in the application of its host crate.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}
