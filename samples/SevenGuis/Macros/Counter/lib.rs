//! Counter: a read-only text field with a number and a button that counts.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-counter-macros`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;

/// The counter.
#[component]
pub fn Counter() -> Element {
    let count = signal(0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, vertical_alignment: VerticalAlignment::Top, spacing: 8.0, margin: 16.0,
            TextBox { name: "Count", text: count, is_read_only: true, width: 80.0 }
            Button { name: "Increment", on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
