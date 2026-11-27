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
//!
//! `FRAMES_RENDERING` asks for one rendering mode of the window, alone (no fallback, so a mode that
//! does not work fails the run), for the window suite of a backend that is not the platform's
//! default: on Linux `glx`, `egl`, `vulkan` or `software`; on Windows `angle`, `wgl`, `vulkan`,
//! `d3d11` or `software`. Unset, the platform's own order applies. It prints
//! `PERF rendering <mode>` when it is set.

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
    let builder = rendering_options(builder);
    let exit_code = builder.use_platform_detect().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}

/// The rendering mode `FRAMES_RENDERING` names, on the X11 backend.
#[cfg(target_os = "linux")]
fn rendering_options(builder: ferroui_controls::AppBuilder) -> ferroui_controls::AppBuilder {
    use ferroui_x11::{X11PlatformOptions, X11RenderingMode};

    let mode = match std::env::var("FRAMES_RENDERING").ok().as_deref() {
        Some("glx") => X11RenderingMode::Glx,
        Some("egl") => X11RenderingMode::Egl,
        Some("vulkan") => X11RenderingMode::Vulkan,
        Some("software") => X11RenderingMode::Software,
        Some(other) => panic!("FRAMES_RENDERING: `{other}` is not glx, egl, vulkan or software"),
        None => return builder,
    };
    println!("PERF rendering {mode:?}");
    let mut options = X11PlatformOptions::new();
    options.rendering_mode = vec![mode];
    builder.with(std::rc::Rc::new(options))
}

/// The rendering mode `FRAMES_RENDERING` names, on the Win32 backend.
#[cfg(windows)]
fn rendering_options(builder: ferroui_controls::AppBuilder) -> ferroui_controls::AppBuilder {
    use ferroui_win32::{Win32PlatformOptions, Win32RenderingMode};

    let mode = match std::env::var("FRAMES_RENDERING").ok().as_deref() {
        Some("angle") => Win32RenderingMode::AngleEgl,
        Some("wgl") => Win32RenderingMode::Wgl,
        Some("vulkan") => Win32RenderingMode::Vulkan,
        Some("d3d11") => Win32RenderingMode::Direct3D11,
        Some("software") => Win32RenderingMode::Software,
        Some(other) => panic!("FRAMES_RENDERING: `{other}` is not angle, wgl, vulkan, d3d11 or software"),
        None => return builder,
    };
    println!("PERF rendering {mode:?}");
    let options = Win32PlatformOptions { rendering_mode: vec![mode], ..Default::default() };
    builder.with(std::rc::Rc::new(options))
}

#[cfg(not(any(target_os = "linux", windows, target_os = "android", target_os = "ios")))]
fn rendering_options(builder: ferroui_controls::AppBuilder) -> ferroui_controls::AppBuilder {
    builder
}

/// On a device the scene runs in the application of its host crate.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn main() {}
