//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-circle-drawer", "Circle Drawer", 560.0, 460.0, || {
        ferroui_view::IntoElement::into_element(seven_guis_circle_drawer_hooks::circle_drawer())
    });
}
