//! The desktop entry point of the HelloWorld sample.
//!
//! ```text
//! cargo run -p hello-world
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world
//! cargo build -p hello-world --profile dist
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which ends
//! the main loop (`sample_support::smoke_run`).

use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use hello_world::App;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(all(target_os = "macos", feature = "quartz"))]
    let builder = builder.with(std::rc::Rc::new(
        ferroui_native::FerroNativePlatformOptions {
            rendering_mode: vec![if cfg!(feature = "quartz-metal") {
                ferroui_native::FerroNativeRenderingMode::Metal
            } else {
                ferroui_native::FerroNativeRenderingMode::Software
            }],
            ..Default::default()
        },
    ));
    let exit_code = builder
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .start_with_classic_desktop_lifetime(&args);
    #[cfg(all(target_os = "macos", feature = "quartz-metal"))]
    if std::env::var_os("FERROUI_METAL_STATS").is_some() {
        eprintln!(
            "Quartz Metal [surfaces, paths, images, presents, readbacks, uploads, errors]: {:?}",
            ferroui_quartz::metal_statistics()
        );
    }
    std::process::ExitCode::from(exit_code as u8)
}
