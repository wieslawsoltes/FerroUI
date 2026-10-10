//! Adding a text box to a window whose styles hold a hundred styles with
//! class selectors that do not match it, and applying its template.

use crate::harness::Registry;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Control, TextBox, Window};

pub struct ApplyStyling {
    window: Ref<Window>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ApplyStyling {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(super::styled_window());

        let text_box = TextBox::new();

        let window = Window::new();
        window.set_content(Some(Control::boxed(&text_box)));

        window.apply_template();
        text_box.apply_template();

        let children = text_box.get_visual_children();
        assert!(children.len() == 1, "Sequence contains {} elements, one is expected.", children.len());
        let border = children[0].cast::<Border>().expect("The visual child of the text box is not a border.");

        if border.border_thickness() != Thickness::uniform(2.0) {
            panic!("Styles not applied.");
        }

        window.set_content(None);

        // Add a bunch of styles with lots of class selectors to complicate matters.
        for _ in 0..100 {
            window.styles().add(&Style::with_setters(
                Selectors::of_type::<TextBox>().class("foo").class("bar").class("baz"),
                [Setter::new(TextBox::text_property(), Some("foo".to_string()))],
            ));
        }

        Self { window, _app: app }
    }

    pub fn add_and_style_text_box(&self) {
        let text_box = TextBox::new();
        self.window.set_content(Some(Control::boxed(&text_box)));
        text_box.apply_template();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "ApplyStyling");
    class.benchmark("add_and_style_text_box", "", ApplyStyling::new, |b| b.add_and_style_text_box());
}

#[cfg(test)]
mod tests {
    /// The setup of the upstream benchmark checks that the border of the text box is two units
    /// thick and throws "Styles not applied." otherwise. Its services have the Simple theme as
    /// the theme of the application (`TestServices.StyledWindow`: `new SimpleTheme()`), whose
    /// text box takes `ThemeBorderThickness`, which is 1 at the tracked commit
    /// (`Accents/Base.xaml`): the upstream benchmark fails in its setup, and so does the port,
    /// with the check as it is.
    #[test]
    #[should_panic(expected = "Styles not applied.")]
    fn apply_styling() {
        crate::harness::smoke_class(super::register, "ApplyStyling");
    }
}
