//! The task in a window of its own, in the WinUI theme with the units its controls need.

// `compiled_markup`: the table of the units of the theme the build wrote (`build.rs`).
ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() {
    seven_guis_model::run_with_parts("macros-crud", "CRUD", 600.0, 400.0, THEME_PARTS, seven_guis_crud_macros::Crud);
}
