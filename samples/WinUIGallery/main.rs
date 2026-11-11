//! `cargo run -p winui-gallery --features desktop -- --page=Button`.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<winui_gallery::App>().use_platform_detect();
    #[cfg(target_os = "linux")]
    let builder = {
        use ferroui_x11::{X11PlatformOptions, X11RenderingMode};
        let mut options = X11PlatformOptions::new();
        options.rendering_mode.insert(0, X11RenderingMode::Egl);
        builder.with(std::rc::Rc::new(options))
    };
    #[cfg(all(target_os = "macos", feature = "quartz-metal"))]
    let builder = {
        let mut options = ferroui_native::FerroNativePlatformOptions::default();
        options.rendering_mode = vec![ferroui_native::FerroNativeRenderingMode::Metal];
        builder.with(std::rc::Rc::new(options))
    };
    let code = builder
        .log_to_trace(LogEventLevel::Warning, &[])
        .start_with_classic_desktop_lifetime(&arguments);
    std::process::ExitCode::from(code.clamp(0, 255) as u8)
}
