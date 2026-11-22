use ferroui_controls::{templates::IDataTemplate, Border, TextBlock};
use ferroui_view::prelude::*;
use std::rc::Rc;

pub(super) fn queue_row() -> Rc<dyn IDataTemplate> {
    ::ferroui_view::template(|customer: String| ::ferroui_view::rsx! {
        Border { background: "#EEF3FB", corner_radius: 6.0, padding: 8.0,
            TextBlock { text: customer, foreground: "#203657" }
        }
    })
}
