//! Temperature Converter: two text fields, Celsius and Fahrenheit; typing a
//! number into one converts it into the other.
//!
//! Written as a component that runs again, with its state in hooks; `cargo run -p seven-guis-temperature-hooks`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use seven_guis_model::{celsius_to_fahrenheit, fahrenheit_to_celsius, format_degrees, parse_degrees};

/// The converter. Each field shows its state and writes what the user typed
/// back into it, as a controlled input of React does; a field whose text is
/// already the state changes nothing, so the other field's conversion is not
/// converted back.
#[component(rerender)]
pub fn TemperatureConverter() -> Element {
    let celsius = use_signal(String::new);
    let fahrenheit = use_signal(String::new);
    let typed = move |field: Signal<String>, other: Signal<String>, convert: fn(f64) -> f64| {
        move |text: &Option<String>| {
            let text = text.clone().unwrap_or_default();
            if field.get_untracked() == text {
                return;
            }
            if let Some(value) = parse_degrees(&text) {
                other.set(format_degrees(convert(value)));
            }
            field.set(text);
        }
    };

    rsx! {
        StackPanel { orientation: Orientation::Horizontal, vertical_alignment: VerticalAlignment::Top, spacing: 8.0, margin: 16.0,
            TextBox { name: "Celsius", width: 80.0, text: celsius.get(),
                on_changed(TextBox::text_property(), typed(celsius, fahrenheit, celsius_to_fahrenheit)) }
            TextBlock { text: "Celsius =", vertical_alignment: VerticalAlignment::Center }
            TextBox { name: "Fahrenheit", width: 80.0, text: fahrenheit.get(),
                on_changed(TextBox::text_property(), typed(fahrenheit, celsius, fahrenheit_to_celsius)) }
            TextBlock { text: "Fahrenheit", vertical_alignment: VerticalAlignment::Center }
        }
    }
}
