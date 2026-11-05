//! The desktop entry point of the custom effects gallery: the gallery in a window of its own, in
//! the WinUI 3 theme, its page transitions sampling pages it captures.
//!
//! ```text
//! cargo run -p custom-effects
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p custom-effects
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the window is closed after `n` milliseconds, which ends the
//! main loop.

use custom_effects::TransitionImages;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    AppBuilder::configure::<Application>().use_platform_detect().start(
        move |app, _| {
            app.styles().add(ferroui_themes_winui::WinUITheme::new().as_style());
            // The pages are captured once the platform has its render interface.
            let window = custom_effects::main_window(&TransitionImages::render());
            if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) {
                let closing = window.clone();
                let _timer = DispatcherTimer::run_once(move || closing.close(), Duration::from_millis(ms), DispatcherPriority::NORMAL)
                    .detach();
            }
            app.run_window(window);
        },
        &args,
    );
}
