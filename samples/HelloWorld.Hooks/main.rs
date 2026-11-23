//! The window of the sample: `cargo run -p hello-world-hooks` (`FERROUI_SMOKE_EXIT_MS=<n>` closes it after
//! `n` milliseconds).

use ferroui_view::prelude::*;

// `compiled_markup`: the table of the units of the theme the build wrote (`build.rs`).
ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() {
    launch_themed(
        || ferroui_themes_winui::WinUITheme::with_parts(None, THEME_PARTS).as_style(),
        || window().title("Hello, World!").width(400.0).height(240.0).child(IntoElement::into_element(hello_world_hooks::hello())).into(),
        |_| {},
    );
}
