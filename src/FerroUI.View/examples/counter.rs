//! The counter of the README (`## Declarative Rust UI`), kept building:
//! `cargo run -p ferroui-view --features desktop --example counter`.

use ferroui_base::layout::Orientation;
use ferroui_view::prelude::*;

#[component(rerender)]
fn Counter() -> Element {
    let count = use_signal(|| 0);
    rsx! {
        StackPanel { orientation: Orientation::Horizontal, spacing: 8.0,
            TextBlock { text: format!("Clicked {} times", count.get()) }
            Button { class: "accent", on_click: move |_| count.update(|c| *c += 1), "Click" }
        }
    }
}

fn main() {
    ferroui_view::launch(|| rsx! { Window { title: "Counter", Counter {} } });
}
