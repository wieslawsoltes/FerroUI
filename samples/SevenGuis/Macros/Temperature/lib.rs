//! Temperature converter: two text fields, degrees Celsius and Fahrenheit;
//! typing a number into one sets the other, typing anything else leaves the
//! other as it is.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-temperature-macros`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use seven_guis_model::{celsius_to_fahrenheit, fahrenheit_to_celsius, format_degrees, parse_degrees};
use std::cell::Cell;
use std::rc::Rc;

/// The converter.
#[component]
pub fn TemperatureConverter() -> Element {
    let celsius = signal(String::new());
    let fahrenheit = signal(String::new());
    // A field that the other one set is not converted back, so that the text
    // the user typed stays as it is.
    let setting = Rc::new(Cell::new(false));
    let convert = move |to: Signal<String>, convert: fn(f64) -> f64| {
        let setting = setting.clone();
        move |text: &Option<String>| {
            if setting.get() {
                return;
            }
            if let Some(value) = text.as_deref().and_then(parse_degrees) {
                setting.set(true);
                to.set(format_degrees(convert(value)));
                setting.set(false);
            }
        }
    };
    let to_fahrenheit = convert(fahrenheit, celsius_to_fahrenheit);
    let to_celsius = convert(celsius, fahrenheit_to_celsius);

    rsx! {
        StackPanel { orientation: Orientation::Horizontal, vertical_alignment: VerticalAlignment::Top, spacing: 8.0, margin: 16.0,
            TextBox { name: "Celsius", width: 80.0, text: celsius,
                on_changed(TextBox::text_property(), to_fahrenheit) }
            TextBlock { text: "Celsius =", vertical_alignment: VerticalAlignment::Center }
            TextBox { name: "Fahrenheit", width: 80.0, text: fahrenheit,
                on_changed(TextBox::text_property(), to_celsius) }
            TextBlock { text: "Fahrenheit", vertical_alignment: VerticalAlignment::Center }
        }
    }
}
