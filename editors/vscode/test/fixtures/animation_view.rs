use ferroui_view::prelude::*;

const HOLD_CUE: f64 = 0.25;
const HOLD_OPACITY: f64 = 0.65;
const HOLD_Y2: f64 = 1.0;

fn headline() -> Element {
    rsx! { TextBlock { text: "Forecast · 🪴" } }
}

fn forecast() -> Element {
    rsx! {
        Border { background: "#EEF2FB", padding: 32.0,
            StackPanel { spacing: 16.0,
                styles: [
                    ferroui_view::style("Border#Forecast").animation(
                        ferroui_view::animation(ferroui_view::ms(2000))
                            .key_frame(0.0, |key| key.set(ferroui_base::Visual::opacity_property(), 0.25))
                            .key_frame(HOLD_CUE, |key| key.set(ferroui_base::Visual::opacity_property(), /* 🪴 hold */ HOLD_OPACITY)
                                .key_spline(0.25, 0.1, 0.75, HOLD_Y2))
                            .key_frame(1.0, |key| key.set(ferroui_base::Visual::opacity_property(), 1.0))
                            .fill_mode(ferroui_base::animation::FillMode::Both)
                    ),
                ],
                TextBlock { text: "Revenue forecast", font_size: 27.0, foreground: "#142747" }
                Border { name: "Forecast", background: "#FFFFFF", corner_radius: 12.0, padding: 22.0, opacity: 0.25,
                    StackPanel { spacing: 8.0,
                        TextBlock { text: "Expected this quarter", font_size: 14.0, foreground: "#42547A" }
                        TextBlock { text: "$248,500", font_size: 32.0, foreground: "#0D4C6C" }
                    }
                }
                TextBlock { text: forecast_label(), font_size: 14.0 }
            }
        }
    }
}

fn forecast_label() -> &'static str { "Forecast updated from approved contracts" }
