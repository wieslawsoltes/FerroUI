//! `cargo run -p winui-gallery --features desktop -- --page=Button`.

use ferroui_base::logging::LogEventLevel;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let code = AppBuilder::configure::<winui_gallery::App>()
        .use_platform_detect()
        .log_to_trace(LogEventLevel::Warning, &[])
        .start_with_classic_desktop_lifetime(&arguments);
    std::process::ExitCode::from(code.clamp(0, 255) as u8)
}
