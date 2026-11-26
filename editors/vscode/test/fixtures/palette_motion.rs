use ferroui_base::{animation::FillMode, media::Color};
use ferroui_controls::Border;
use ferroui_view::prelude::*;

mod brand {
    pub const INK: u32 = 0xFF27_4676_u32;
}

fn order_confirmation() -> Element {
    rsx! {
        Border { background: "#F0F4FC", padding: 36.0,
            StackPanel { spacing: 18.0,
                styles: [style("Border#Receipt").animation(animation(ms(2000))
                    .key_frame(0.0, |key| key.set(
                        Border::background_property(),
                        Color::from_uint32(brand::INK),
                    ))
                    .key_frame(1.0, |key| key.set(
                        Border::background_property(),
                        Color::from_uint32(brand::/* end ink 🎨 */ INK),
                    ))
                    .fill_mode(FillMode::Both))],
                TextBlock { text: "Order confirmation", font_size: 32.0, foreground: "#132D54" }
                Border { name: "Receipt", background: "#274676", corner_radius: 16.0, padding: 24.0,
                    StackPanel { spacing: 5.0,
                        TextBlock { text: "✓  Order confirmed", font_size: 25.0, foreground: "#FFFFFF" }
                        TextBlock { text: "A receipt is on its way to your inbox.", font_size: 16.0, foreground: "#F3F7FF" }
                    }
                }
            }
        }
    }
}
