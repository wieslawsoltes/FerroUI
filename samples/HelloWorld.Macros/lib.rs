//! Hello, World! written with the `rsx!` macro and `#[component]` of `ferroui-view`
//! (docs/declarative-ui.md, section 5): the text and a button that counts its clicks, the count a
//! signal (section 6) that the text of the button follows.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_view::prelude::*;

/// The content of the window.
#[component]
pub fn Hello() -> Element {
    let clicks = signal(0);
    rsx! {
        StackPanel { horizontal_alignment: HorizontalAlignment::Center, vertical_alignment: VerticalAlignment::Center, spacing: 12.0,
            TextBlock { text: "Hello, World!", font_size: 28.0 }
            Button { name: "Count", horizontal_alignment: HorizontalAlignment::Center, on_click: move |_| clicks.update(|c| *c += 1),
                TextBlock { text: move || clicked_text(clicks.get()) }
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
