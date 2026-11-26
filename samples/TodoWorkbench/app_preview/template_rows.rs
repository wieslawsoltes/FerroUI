use ferroui_controls::{templates::IDataTemplate, Border, TextBlock, TextBox};
use ferroui_view::prelude::*;
use std::rc::Rc;

pub(super) fn queue_row() -> Rc<dyn IDataTemplate> {
    ::ferroui_view::template(|customer: String| ::ferroui_view::rsx! {
        Border { background: "#EEF3FB", corner_radius: 6.0, padding: 8.0,
            TextBlock { text: customer, foreground: "#203657" }
        }
    })
}

pub(super) fn review_editor() -> Rc<dyn IDataTemplate> {
    ::ferroui_view::template! {
        |_: String| ::ferroui_view::rsx! {
            TextBox { placeholder_text: "Review status" }
        }
    }
}
