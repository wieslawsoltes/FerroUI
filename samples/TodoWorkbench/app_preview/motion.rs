use ferroui_base::{CornerRadius, Ref, StyledProperty};
use ferroui_controls::Control;
use ferroui_view::prelude::*;

pub(crate) fn review_corner_property() -> &'static StyledProperty<CornerRadius> {
    ferroui_controls::Border::corner_radius_property()
}

pub(super) fn view() -> Ref<Control> {
    rsx! {
        StackPanel {
            width: 520.0,
            height: 340.0,
            margin: 24.0,
            spacing: 16.0,
            styles: [
                ferroui_view::style("Border#ReviewCard").animation(
                    ferroui_view::animation(ferroui_view::ms(2000))
                        .key_frame(0.0, |key| key.set(
                            crate::app_preview::motion::review_corner_property(),
                            ferroui_base::CornerRadius::new(0.0, 0.0, 0.0, 0.0),
                        ))
                        .key_frame(1.0, |key| key.set(
                            crate::app_preview::motion::review_corner_property(),
                            ferroui_base::CornerRadius::new(32.0, 32.0, 32.0, 32.0),
                        ))
                        .fill_mode(ferroui_base::animation::FillMode::Both)
                ),
            ],
            TextBlock { text: "Review workspace", font_size: 28.0, foreground: "#203657" }
            Border {
                name: "ReviewCard",
                width: 350.0,
                height: 152.0,
                horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left,
                background: "#234976",
                padding: 20.0,
                StackPanel {
                    spacing: 8.0,
                    TextBlock { text: "IN REVIEW", font_size: 13.0, foreground: "#ADCAF4" }
                    TextBlock { text: "Priority accounts", font_size: 25.0, foreground: "#FFFFFF" }
                    TextBlock { text: "12 accounts waiting for follow-up", foreground: "#D3E5FA" }
                }
            }
        }
    }.mount()
}
