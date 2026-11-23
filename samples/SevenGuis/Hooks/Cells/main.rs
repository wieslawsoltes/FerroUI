//! The task in a window of its own, in the WinUI theme with the units its controls need.

// `compiled_markup`: the table of the units of the theme the build wrote (`build.rs`).
ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() {
    seven_guis_model::run_with_parts("hooks-cells", "Cells", 900.0, 600.0, THEME_PARTS, || {
        ferroui_view::IntoElement::into_element(seven_guis_cells_hooks::cells())
    });
}
