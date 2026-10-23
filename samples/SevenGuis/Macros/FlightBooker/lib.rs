//! Flight booker: a one-way or a return flight, two dates (`dd.mm.yyyy`),
//! a date that does not parse is marked, the return date is used for a
//! return flight only, and the booking needs valid dates in order.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-flight-booker-macros`.

use ferroui_base::media::{Brush, IBrush};
use ferroui_view::prelude::*;
use seven_guis_model::{booking_message, can_book, Date, FlightKind};
use std::rc::Rc;

/// The background of a date that does not parse.
fn invalid_background() -> Option<Rc<dyn IBrush>> {
    Brush::parse("#FFC8C8").ok()
}

/// The flight booker.
#[component]
pub fn FlightBooker() -> Element {
    let kind_index = signal(0_i32);
    let start = signal(String::from("27.03.2014"));
    let back = signal(String::from("27.03.2014"));
    let message = signal(String::new());

    let kind = memo(move |_| FlightKind::ALL[kind_index.get().clamp(0, 1) as usize]);
    let start_date = memo(move |_| Date::parse(&start.get()));
    let back_date = memo(move |_| Date::parse(&back.get()));
    let bookable = memo(move |_| can_book(kind.get(), start_date.get(), back_date.get()));
    let book = move |_: &_| {
        if let Some(start) = start_date.get_untracked() {
            message.set(booking_message(kind.get_untracked(), start, back_date.get_untracked()));
        }
    };

    rsx! {
        StackPanel { spacing: 8.0, margin: 16.0, width: 240.0,
            ComboBox { name: "Kind", selected_index: kind_index.two_way(),
                for kind in FlightKind::ALL { {kind.label()} }
            }
            TextBox { name: "Start", text: start.two_way(),
                background: move || if start_date.get().is_some() { None } else { invalid_background() } }
            TextBox { name: "Return", text: back.two_way(),
                background: move || if back_date.get().is_some() { None } else { invalid_background() },
                is_enabled: move || kind.get() == FlightKind::Return }
            Button { name: "Book", is_enabled: bookable, on_click: book, "Book" }
            TextBlock { name: "Message", text: message }
        }
    }
}
