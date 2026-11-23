use ferroui_base::{animation::easings::CubicEaseOut, layout::Layoutable, Visual};
use ferroui_view::prelude::*;
use ferroui_view::{transition as fade, transition_eased, transitions as collection};

fn headline() -> Element {
    rsx! { TextBlock { text: "Account dashboard" } }
}

fn account_card() -> Element {
    rsx! {
        Border {
            name: "AccountCard", background: "#F4F8F6", padding: 26.0,
            transitions: collection([
                fade(Visual::opacity_property(), ferroui_view::ms(/* hold 🌱 */ 800u64)),
                transition_eased(Layoutable::margin_property(),
                    std::time::Duration::from_millis(300), CubicEaseOut::new()),
            ]),
            styles: [
                ferroui_view::style("Border#AccountCard:pointerover")
                    .set(Visual::opacity_property(), 0.55f64),
            ],
            TextBlock { text: "Latest renewals", font_size: 25.0 }
        }
    }
}
