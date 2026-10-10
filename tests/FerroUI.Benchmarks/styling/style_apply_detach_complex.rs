//! Detaching and applying again the styles of a text box at the bottom of a
//! tree of nested panels that each hold a lot of styles.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::styling::{IStyle, Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref, TypeInfo};
use ferroui_controls::testing::{NullRenderer, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Border, Button, ButtonSpinner, Carousel, CheckBox, ComboBox, ContentControl, Control, Expander, ItemsControl,
    Label, ListBox, Panel, ProgressBar, RadioButton, RepeatButton, ScrollViewer, Slider, Spinner, SplitView, TextBox,
    ToggleSwitch, TreeView, Viewbox, Window,
};
use std::rc::Rc;

pub struct StyleApplyDetachComplex {
    /// The tree the control is in: a control does not keep its parents.
    _root: Ref<TestRoot>,
    control: Ref<TextBox>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl StyleApplyDetachComplex {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(super::styled_window());

        // Simulate an application with a lot of styles by creating a tree of nested panels,
        // each with a bunch of styles applied.
        let (root_panel, leaf_panel) = Self::create_nested_panels(10);

        // We're benchmarking how long it takes to apply styles to a TextBox in this situation.
        let control = TextBox::new();
        leaf_panel.children().add(&control);

        let root = TestRoot::with_global_styles(true, Some(root_panel.upcast()));
        root.set_renderer(NullRenderer::new());

        Self { _root: root, control, _app: app }
    }

    pub fn apply_detach_styles(&self) {
        // Styles will have already been attached when attached to the logical tree, so remove
        // the styles first.
        if tag_text(&self.control.tag()) != Some("TextBox") {
            panic!("Invalid benchmark state");
        }

        self.control.invalidate_styles(true);

        if self.control.tag().is_some() {
            panic!("Invalid benchmark state");
        }

        // Then re-apply the styles.
        self.control.apply_styling();
    }

    fn create_nested_panels(count: i32) -> (Ref<Panel>, Ref<Panel>) {
        let root = Panel::new();
        let mut last = root.clone();

        for _ in 0..count {
            let panel = Panel::new();
            panel.styles().add_range(Self::create_styles());
            last.children().add(&panel);
            last = panel;
        }

        (root, last)
    }

    fn create_styles() -> Vec<Rc<dyn IStyle>> {
        let types: [&'static TypeInfo; 23] = [
            Border::TYPE,
            Button::TYPE,
            ButtonSpinner::TYPE,
            Carousel::TYPE,
            CheckBox::TYPE,
            ComboBox::TYPE,
            ContentControl::TYPE,
            Expander::TYPE,
            ItemsControl::TYPE,
            Label::TYPE,
            ListBox::TYPE,
            ProgressBar::TYPE,
            RadioButton::TYPE,
            RepeatButton::TYPE,
            ScrollViewer::TYPE,
            Slider::TYPE,
            Spinner::TYPE,
            SplitView::TYPE,
            TextBox::TYPE,
            ToggleSwitch::TYPE,
            TreeView::TYPE,
            Viewbox::TYPE,
            Window::TYPE,
        ];

        let mut styles: Vec<Rc<dyn IStyle>> = Vec::with_capacity(types.len() * 3);

        for type_ in types {
            styles.push(
                Style::with_setters(
                    Selectors::of_type_info(None, type_),
                    [Setter::new(Control::tag_property(), tag(type_.name().to_string()))],
                )
                .into(),
            );

            styles.push(
                Style::with_setters(
                    Selectors::of_type_info(None, type_).class("foo"),
                    [Setter::new(Control::tag_property(), tag(format!("{} foo", type_.name())))],
                )
                .into(),
            );

            styles.push(
                Style::with_setters(
                    Selectors::of_type_info(None, type_).class("bar"),
                    [Setter::new(Control::tag_property(), tag(format!("{} bar", type_.name())))],
                )
                .into(),
            );
        }

        styles
    }
}

/// A text as the value of the tag of a control.
fn tag(text: String) -> Option<BoxedValue> {
    let value: BoxedValue = Rc::new(text);
    Some(value)
}

/// The text the tag of a control holds, if it holds one.
fn tag_text(tag: &Option<BoxedValue>) -> Option<&str> {
    tag.as_ref().and_then(|value| value.downcast_ref::<String>()).map(String::as_str)
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "StyleApplyDetachComplex");
    class.benchmark("apply_detach_styles", "", StyleApplyDetachComplex::new, |b| b.apply_detach_styles());
}

#[cfg(test)]
mod tests {
    #[test]
    fn style_apply_detach_complex() {
        crate::harness::smoke_class(super::register, "StyleApplyDetachComplex");
    }
}
