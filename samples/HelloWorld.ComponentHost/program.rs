//! The desktop entry point of the HelloWorld sample with a component (`hello-world-component-host`).
//!
//! ```text
//! cargo run -p hello-world-component-host
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p hello-world-component-host
//! cargo build -p hello-world-component-host --profile dist
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n` milliseconds, which ends
//! the main loop (`sample_support::smoke_run`).

use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
use hello_world_component_host::App;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let exit_code = AppBuilder::configure::<App>()
        .use_platform_detect()
        .after_setup(|_| sample_support::smoke_run())
        .start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
