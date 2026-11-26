use ferroui_base::styling::ThemeVariant as Appearance;
use ferroui_controls::TextBlock as Label;
use ferroui_view::{Element, rsx, style as rule};

mod brand {
    use ferroui_base::media::Color as Rgb;

    pub const INK_KEY: &str = "ReviewInk";
    pub const REVIEW_INK: Rgb = DEFAULT_INK;
    pub const NIGHT_INK: Rgb = Rgb::from_rgb(32, 92, 82);
    const DEFAULT_INK: Rgb = Rgb::from_rgb(37u8, /* brand 🧭 */ 74, 112);
}

fn overview() -> Element {
    rsx! { TextBlock { name: "Overview", text: "Account overview" } }
}

fn review() -> Element {
    use self::brand::{INK_KEY, NIGHT_INK, REVIEW_INK as HEADING};

    rsx! {
        Border { background: "#F1F5FA", padding: 34.0,
            StackPanel { spacing: 12.0,
                styles: [rule("TextBlock.review")
                    .theme_resource(Appearance::default(), INK_KEY, (/* account palette */ HEADING))
                    .theme_resource(Appearance::dark(), INK_KEY, NIGHT_INK)
                    .set_resource(Label::foreground_property(), INK_KEY)],
                TextBlock { name: "SharedStatus", classes: "review", text: "Account review", font_size: 28.0 }
                Border { background: "White", padding: 20.0, corner_radius: 10.0,
                    StackPanel { spacing: 7.0,
                        TextBlock { classes: "review", text: "Northwind Studio", font_size: 18.0 }
                        TextBlock { text: "The account is ready for review", foreground: "#4D6480" }
                    }
                }
            }
        }
    }
}
