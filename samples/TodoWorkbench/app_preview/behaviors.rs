//! The same authored behaviors run in the Rust app and its design projection.
use ferroui_view::behavior::{add_class, change_property, event, remove_class};
use ferroui_view::prelude::*;

pub(super) fn view() -> Element {
    rsx! {
        Border {
            width: 540.0,
            padding: 30.0,
            corner_radius: 16.0,
            background: "#EFF4FD",
            StackPanel {
                spacing: 16.0,
                TextBlock { text: "Release approval", font_size: 28.0, foreground: "#15294B" }
                TextBlock {
                    text: "Review the package before it goes to the release team.",
                    foreground: "#405576",
                }
                Border {
                    name: "ApprovalCard",
                    background: "#FFFFFF",
                    corner_radius: 12.0,
                    padding: 20.0,
                    TextBlock { name: "ApprovalStatus", text: "Waiting for review", foreground: "#183A67" }
                }
                StackPanel {
                    orientation: ferroui_base::layout::Orientation::Horizontal,
                    spacing: 10.0,
                    Button {
                        name: "Approve",
                        content: "Approve package",
                        behaviors: [
                            event("Click", None, [
                                change_property(Some("ApprovalStatus"), "Text", "Ready for handoff"),
                                add_class(Some("ApprovalCard"), "approved"),
                            ]),
                        ],
                    }
                    Button {
                        name: "Reopen",
                        content: "Reopen review",
                        behaviors: [
                            event("Click", None, [
                                change_property(Some("ApprovalStatus"), "Text", "Waiting for review"),
                                remove_class(Some("ApprovalCard"), "approved"),
                            ]),
                        ],
                    }
                }
            }
        }
    }
}
