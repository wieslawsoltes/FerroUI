//! The window of the sample: `cargo run -p hello-world-macros` (`FERROUI_SMOKE_EXIT_MS=<n>` closes it after
//! `n` milliseconds).

use ferroui_view::prelude::*;

fn main() {
    launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        || window().title("Hello, World!").width(400.0).height(240.0).child(hello_world_macros::Hello()).into(),
        |_| {},
    );
}
