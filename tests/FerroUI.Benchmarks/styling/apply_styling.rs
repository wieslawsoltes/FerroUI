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

        // The theme style sets the border thickness of the text box (the Simple theme takes
        // `ThemeBorderThickness`), and the template binds the border to it: a border without
        // the thickness of its text box, or with the default one, means the styles were not
        // applied.
        let thickness = border.border_thickness();
        if thickness == Thickness::default() || thickness != text_box.border_thickness() {
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
    /// The setup checks that the theme styled the text box and that its template carried the
    /// border thickness to the border. The upstream benchmark expects two units, which is not
    /// the thickness of the Simple theme its services use (`ThemeBorderThickness` is 1), so it
    /// fails in its setup; the port checks the styled value instead, which keeps the class
    /// runnable by the gate.
    #[test]
    fn apply_styling() {
        crate::harness::smoke_class(super::register, "ApplyStyling");
    }
}
