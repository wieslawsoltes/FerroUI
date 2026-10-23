//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-counter", "Counter", 320.0, 110.0, || ferroui_view::IntoElement::into_element(seven_guis_counter_hooks::counter()));
}
