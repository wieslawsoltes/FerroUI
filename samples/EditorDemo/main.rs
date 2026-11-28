use editor_demo::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
#[cfg(feature = "headless")]
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};

#[cfg(any(feature = "desktop", feature = "direct2d", feature = "headless", all(target_os = "linux", feature = "wayland")))]
fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(feature = "headless")]
    let builder = builder.use_headless(FerroHeadlessPlatformOptions::default());
    #[cfg(all(target_os = "linux", feature = "wayland", not(feature = "headless")))]
    let builder = {
        use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
        use ferroui_skia::SkiaApplicationExtensions;
        use ferroui_wayland::{FerroWaylandPlatformExtensions, WaylandPlatformOptions};
        let mut options = WaylandPlatformOptions::new();
        options.app_id = Some("org.ferroui.EditorDemo".to_owned());
        builder.with(std::rc::Rc::new(options)).use_harfbuzz().use_skia().use_wayland()
    };
    #[cfg(all(
        any(feature = "desktop", feature = "direct2d"),
        not(any(feature = "headless", all(target_os = "linux", feature = "wayland")))
    ))]
    let builder = {
        use ferroui_desktop::AppBuilderDesktopExtensions;
        builder.use_platform_detect()
    };
    let exit_code = builder
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
        .start_with_classic_desktop_lifetime(&arguments);
    std::process::ExitCode::from(exit_code as u8)
}

#[cfg(not(any(feature = "desktop", feature = "direct2d", feature = "headless", all(target_os = "linux", feature = "wayland"))))]
fn main() -> std::process::ExitCode {
    eprintln!("Enable the desktop, direct2d, headless or Linux wayland feature to run the editor demo.");
    std::process::ExitCode::FAILURE
}
