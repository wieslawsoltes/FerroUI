//! The desktop entry point of the sample, in the WinUI theme with the units its views need.

// `compiled_markup`: the table of the units of the theme the build wrote (`build.rs`).
ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() {
    ferroui_view::launch_themed(
        || ferroui_themes_winui::WinUITheme::with_parts(None, THEME_PARTS).as_style(),
        static_view_locator_demo::main_window,
        |_| {},
    );
}
