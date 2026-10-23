//! Counter: a read-only text field with a number and a button that counts.
//!
//! Written with the builders of `ferroui-view`; `cargo run -p seven-guis-counter-builders`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;

/// The counter.
pub fn counter() -> Element {
    let count = signal(0);
    stack_panel()
        .orientation(Orientation::Horizontal)
        .vertical_alignment(VerticalAlignment::Top)
        .spacing(8.0)
        .margin(16.0)
        .child(text_box().name("Count").text(count).is_read_only(true).width(80.0))
        .child(button().name("Increment").content("Count").on_click(move |_| count.update(|c| *c += 1)))
        .into()
}
