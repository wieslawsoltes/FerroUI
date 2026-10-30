//! Hello, World! written as a component that runs again, with its state in a hook
//! (`#[component(rerender)]`, `use_signal`; docs/declarative-ui.md, section 7): the function runs
//! again when the count changes and the reconciler patches the text of the button.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_view::prelude::*;

/// The content of the window: it runs again when the count changes.
#[component(rerender)]
pub fn Hello() -> Element {
    let clicks = use_signal(|| 0);
    rsx! {
        StackPanel { horizontal_alignment: HorizontalAlignment::Center, vertical_alignment: VerticalAlignment::Center, spacing: 12.0,
            TextBlock { text: "Hello, World!", font_size: 28.0 }
            Button { name: "Count", horizontal_alignment: HorizontalAlignment::Center, on_click: move |_| clicks.update(|c| *c += 1),
                { clicked_text(clicks.get()) }
            }
        }
    }
}

/// The text of the button after `clicks` clicks.
fn clicked_text(clicks: i32) -> String {
    match clicks {
        1 => "Clicked 1 time".to_string(),
        _ => format!("Clicked {clicks} times"),
    }
}
