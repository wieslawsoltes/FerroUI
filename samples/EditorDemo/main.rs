use editor_demo::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
#[cfg(all(feature = "desktop", not(feature = "headless")))]
use ferroui_desktop::AppBuilderDesktopExtensions;
#[cfg(feature = "headless")]
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};

#[cfg(any(feature = "desktop", feature = "headless"))]
fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(feature = "headless")]
    let builder = builder.use_headless(FerroHeadlessPlatformOptions::default());
    #[cfg(all(feature = "desktop", not(feature = "headless")))]
    let builder = builder.use_platform_detect();
    let exit_code = builder
        .after_setup(|_| sample_support::smoke_run())
        .log_to_trace(LogEventLevel::Warning, &[])
        .start_with_classic_desktop_lifetime(&arguments);
    std::process::ExitCode::from(exit_code as u8)
}

#[cfg(not(any(feature = "desktop", feature = "headless")))]
fn main() -> std::process::ExitCode {
    eprintln!("Enable the desktop or headless feature to run the editor demo.");
    std::process::ExitCode::FAILURE
}
