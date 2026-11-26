use ferroui_view::prelude::*;

pub fn workspace_settings() -> Element {
    rsx! {
        UserControl {
            Border { background: "#F3F6FB", padding: 36.0,
                Border { width: 420.0, padding: 28.0, background: "White",
                    border_brush: "#DBE2ED", border_thickness: 1.0, corner_radius: 14.0,
                    horizontal_alignment: ::ferroui_base::layout::HorizontalAlignment::Center,
                    vertical_alignment: ::ferroui_base::layout::VerticalAlignment::Center,
                    StackPanel { spacing: 14.0,
                        TextBlock { font_size: 26.0, foreground: "#1B2C47", text: "Workspace settings" }
                        TextBlock { foreground: "#485E7D", text: "Choose a name for this workspace." }
                        TextBox { text: "Northwind" }
                        Button { name: "saveButton", content: "Save changes",
                            horizontal_alignment: ::ferroui_base::layout::HorizontalAlignment::Left }
                    }
                }
            }
        }
    }
}

pub fn counter() -> Element {
    let count = signal(0);
    rsx! {
        StackPanel { spacing: 8.0,
            TextBlock { text: move || format!("Count: {}", count.get()) }
            Button { on_click: move |_| count.update(|n| *n += 1), "Increment" }
        }
    }
}
