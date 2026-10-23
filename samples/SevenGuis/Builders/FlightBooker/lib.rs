//! Flight booker: a one-way or a return flight, two dates (`dd.mm.yyyy`),
//! a date that does not parse is marked, the return date is used for a
//! return flight only, and the booking needs valid dates in order.
//!
//! Written with the builders of `ferroui-view`; `cargo run -p seven-guis-flight-booker-builders`.

use ferroui_base::media::{Brush, IBrush};
use ferroui_view::prelude::*;
use seven_guis_model::{booking_message, can_book, Date, FlightKind};
use std::rc::Rc;

/// The background of a date that does not parse.
pub fn invalid_background() -> Option<Rc<dyn IBrush>> {
    Brush::parse("#FFC8C8").ok()
}

/// The flight booker.
pub fn flight_booker() -> Element {
    let kind_index = signal(0_i32);
    let start = signal(String::from("27.03.2014"));
    let back = signal(String::from("27.03.2014"));
    let message = signal(String::new());

    let kind = memo(move |_| FlightKind::ALL[kind_index.get().clamp(0, 1) as usize]);
    let start_date = memo(move |_| Date::parse(&start.get()));
    let back_date = memo(move |_| Date::parse(&back.get()));
    let bookable = memo(move |_| can_book(kind.get(), start_date.get(), back_date.get()));

    let date_box = |name: &str, text: Signal<String>, date: Memo<Option<Date>>| {
        text_box()
            .name(name)
            .text(text.two_way())
            .background(move || if date.get().is_some() { None } else { invalid_background() })
    };

    stack_panel()
        .spacing(8.0)
        .margin(16.0)
        .width(240.0)
        .child(
            combo_box()
                .name("Kind")
                .selected_index(kind_index.two_way())
                .children(FlightKind::ALL.map(FlightKind::label)),
        )
        .child(date_box("Start", start, start_date))
        .child(date_box("Return", back, back_date).is_enabled(move || kind.get() == FlightKind::Return))
        .child(button().name("Book").content("Book").is_enabled(bookable).on_click(move |_| {
            if let Some(start) = start_date.get_untracked() {
                message.set(booking_message(kind.get_untracked(), start, back_date.get_untracked()));
            }
        }))
        .child(text_block().name("Message").text(message))
        .into()
}
