//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-timer", "Timer", 360.0, 300.0, || ferroui_view::IntoElement::into_element(seven_guis_timer_hooks::timer()));
}
