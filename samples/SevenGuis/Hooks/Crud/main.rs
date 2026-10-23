//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-crud", "CRUD", 600.0, 400.0, || ferroui_view::IntoElement::into_element(seven_guis_crud_hooks::crud()));
}
