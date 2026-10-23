//! The task in a window of its own.

fn main() {
    seven_guis_model::run("hooks-temperature", "Temperature Converter", 480.0, 110.0, || {
        ferroui_view::IntoElement::into_element(seven_guis_temperature_hooks::temperature_converter())
    });
}
