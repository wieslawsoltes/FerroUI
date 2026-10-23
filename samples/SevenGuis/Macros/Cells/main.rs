//! The task in a window of its own.

fn main() {
    seven_guis_model::run("macros-cells", "Cells", 900.0, 600.0, || ferroui_view::IntoElement::into_element(seven_guis_cells_macros::cells()));
}
