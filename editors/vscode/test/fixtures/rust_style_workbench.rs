use ferroui_view::prelude::*;

fn customer_review() -> Element {
    rsx! {
        Border { background: "#EDF2FA", padding: 32.0,
            StackPanel { spacing: 14.0,
                styles: [
                    ferroui_view::style(r"#Review")
                        .resource("ReviewInk", ferroui_base::media::Color::from_rgb(34, /* palette */ 78, 124))
                        .set(ferroui_controls::TextBlock::font_size_property(), /* 🧭 headline */ 24.0f64)
                        .set(ferroui_controls::TextBlock::foreground_property(), "#1C3554")
                        .nested(ferroui_view::style("^.muted")
                            .set(ferroui_base::Visual::opacity_property(), 0.7))
                        .animation(ferroui_view::animation(ferroui_view::ms(1200))
                            .key_frame(0.0, |key| key.set(ferroui_base::Visual::opacity_property(), 1.0))
                            .key_frame(1.0, |key| key.set(ferroui_base::Visual::opacity_property(), 1.0))),
                    ferroui_view::style("TextBlock.note")
                        .set(ferroui_controls::TextBlock::font_size_property(), preferred_note_size()),
                ],
                TextBlock { name: "Review", text: "Customer review" }
                Border { background: "#FFFFFF", padding: 20.0, corner_radius: 12.0,
                    StackPanel { spacing: 8.0,
                        TextBlock { text: "Northwind Studio", font_size: 19.0, foreground: "#1C3554" }
                        TextBlock { text: "Ready for your team", classes: "note", foreground: "#41658D" }
                    }
                }
            }
        }
    }
}

fn preferred_note_size() -> f64 {
    14.0
}
