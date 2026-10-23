//! Flight Booker: a one-way or return flight, two dates checked as they are
//! typed, and a button that books when the dates are valid.
//!
//! Written as a component that runs again, with its state in hooks; `cargo run -p seven-guis-flight-booker-hooks`.

use ferroui_base::media::{Brush, IBrush};
use ferroui_view::prelude::*;
use seven_guis_model::{booking_message, can_book, Date, FlightKind};
use std::rc::Rc;

/// The background of a date that does not parse.
fn invalid_background() -> Option<Rc<dyn IBrush>> {
    Brush::parse("#FFC8C8").ok()
}

/// The flight booker. What follows from the state (the kind, the dates,
/// whether it can book) is computed on each run, as plain values.
#[component(rerender)]
pub fn FlightBooker() -> Element {
    let kind_index = use_signal(|| 0_i32);
    let start = use_signal(|| String::from("27.03.2014"));
    let back = use_signal(|| String::from("27.03.2014"));
    let message = use_signal(String::new);

    let kind = FlightKind::ALL[kind_index.get().clamp(0, 1) as usize];
    let start_date = Date::parse(&start.get());
    let back_date = Date::parse(&back.get());
    let book = move |_: &_| {
        if let Some(start_date) = start_date {
            message.set(booking_message(kind, start_date, back_date));
        }
    };

    rsx! {
        StackPanel { spacing: 8.0, margin: 16.0, width: 240.0,
            ComboBox { name: "Kind", selected_index: kind_index.two_way(),
                for kind in FlightKind::ALL { {kind.label()} }
            }
            TextBox { name: "Start", text: start.two_way(),
                background: if start_date.is_some() { None } else { invalid_background() } }
            TextBox { name: "Return", text: back.two_way(),
                background: if back_date.is_some() { None } else { invalid_background() },
                is_enabled: kind == FlightKind::Return }
            Button { name: "Book", is_enabled: can_book(kind, start_date, back_date), on_click: book, "Book" }
            TextBlock { name: "Message", text: message.get() }
        }
    }
}
