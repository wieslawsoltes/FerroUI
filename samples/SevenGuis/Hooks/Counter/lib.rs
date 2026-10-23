//! Counter: a read-only text field with a number and a button that counts.
//!
//! Written as a component that runs again, with its state in a hook; `cargo run -p seven-guis-counter-hooks`.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_view::prelude::*;

/// The counter: it runs again when the count changes.
#[component(rerender)]
pub fn Counter() -> Element {
    let count = use_signal(|| 0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, vertical_alignment: VerticalAlignment::Top, spacing: 8.0, margin: 16.0,
            TextBox { name: "Count", text: count.get().to_string(), is_read_only: true, width: 80.0 }
            Button { name: "Increment", on_click: move |_| count.update(|c| *c += 1), "Count" }
        }
    }
}
