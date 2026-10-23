//! The desktop entry point of the sample.

fn main() {
    ferroui_view::launch_themed(|| ferroui_themes_winui::WinUITheme::new().as_style(), static_view_locator_demo::main_window, |_| {});
}
