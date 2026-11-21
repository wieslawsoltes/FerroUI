use ferroui_base::{interactivity::RoutedEventArgs, Ref};
use ferroui_controls::{Border, Button, Control, StackPanel, TextBlock};
use ferroui_view::prelude::*;

pub(super) fn view() -> Ref<Control> {
    rsx! {
        StackPanel {
            width: 516.0, height: 340.0, margin: 24.0, spacing: 14.0,
            visual_states: [
                ferroui_view::visual_state_group("Review")
                    .state(ferroui_view::visual_state("Queued")
                        .set_on("ReviewCard", ferroui_base::layout::Layoutable::width_property(), 260.0)
                        .set_on("ReviewCard", crate::app_preview::motion::review_corner_property(), ferroui_base::CornerRadius::uniform(8.0)))
                    .state(ferroui_view::visual_state("Ready")
                        .set_on("ReviewCard", ferroui_base::layout::Layoutable::width_property(), 440.0)
                        .set_on("ReviewCard", crate::app_preview::motion::review_corner_property(), ferroui_base::CornerRadius::uniform(26.0)))
                    .transition(None, "Ready", ferroui_view::ms(440), "LinearEasing")
                    .transition(Some("Queued"), "Ready", ferroui_view::ms(640), "CubicEaseIn")
                    .transition(Some("Ready"), "Queued", ferroui_view::ms(380), "QuadraticEaseOut"),
            ],
            TextBlock { text: "Account review", font_size: 30.0, foreground: "#203657" }
            TextBlock { text: "Move a customer between review states.", font_size: 15.0, foreground: "#52678B" }
            StackPanel { orientation: ferroui_base::layout::Orientation::Horizontal, spacing: 8.0,
                Button { name: "Queue", on_click: |event| enter(event, Some("Queued")), "Queued" }
                Button { name: "Ready", on_click: |event| enter(event, Some("Ready")), "Ready" }
                Button { name: "Reset", on_click: |event| enter(event, None), "Reset" }
            }
            Border { name: "ReviewCard", width: 326.0, height: 133.0,
                horizontal_alignment: ferroui_base::layout::HorizontalAlignment::Left,
                corner_radius: 14.0, background: "#234976", padding: 18.0,
                StackPanel { spacing: 8.0,
                    TextBlock { text: "PRIORITY ACCOUNT", font_size: 13.0, foreground: "#ADCAF4" }
                    TextBlock { text: "Northwind Studio", font_size: 23.0, foreground: "#FFFFFF" }
                    TextBlock { text: "Renewal review", foreground: "#D3E5FA" }
                }
            }
        }
    }.mount()
}

fn enter(event: &RoutedEventArgs, state: Option<&str>) {
    let mut source = event.source().and_then(|source| source.cast::<Control>());
    while let Some(control) = source {
        if set_visual_state(&control, "Review", state, true) {
            return;
        }
        source = control.parent().and_then(|parent| parent.cast::<Control>());
    }
}
