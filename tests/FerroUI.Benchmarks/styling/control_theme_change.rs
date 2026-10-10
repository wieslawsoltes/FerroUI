//! Switching the control theme of a text box at the bottom of a tree of
//! nested panels that each hold a lot of styles.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{ControlTheme, IStyle, Selectors, Setter, Style};
use ferroui_base::{BoxedValue, Ref, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::{NullRenderer, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    Application, Border, Button, ButtonSpinner, Carousel, CheckBox, ComboBox, ContentControl, Control, Expander,
    ItemsControl, Label, ListBox, Panel, ProgressBar, RadioButton, RepeatButton, ScrollViewer, Slider, Spinner,
    SplitView, TextBox, ToggleSwitch, TreeView, Viewbox, Window,
};
use std::rc::Rc;

pub struct ControlThemeChange {
    root: Ref<TestRoot>,
    control: Ref<TextBox>,
    theme1: Ref<ControlTheme>,
    theme2: Ref<ControlTheme>,
    /// The red and the green brush of the predefined brushes, as the value
    /// of a background.
    red: Option<Rc<dyn IBrush>>,
    green: Option<Rc<dyn IBrush>>,
    /// Disposed when the benchmark is dropped, after the tree (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ControlThemeChange {
    pub fn new() -> Self {
        let app = UnitTestApplication::start(super::styled_window());

        // Simulate an application with a lot of styles by creating a tree of nested panels,
        // each with a bunch of styles applied.
        let (root_panel, leaf_panel) = Self::create_nested_panels(10);

        // We're benchmarking how long it takes to switch control theme on a TextBox in this
        // situation.
        let application = Application::current().expect("the application of the benchmark");
        let base_theme = application.as_resource_host().find_resource(&ResourceKey::Type(TextBox::TYPE));
        let base_theme = from_markup_value::<Ref<ControlTheme>>(&base_theme)
            .unwrap_or_else(|| panic!("Base TextBox theme not found."));

        let red: Rc<dyn IBrush> = Brushes::red();
        let green: Rc<dyn IBrush> = Brushes::green();

        let theme1 = ControlTheme::for_type::<TextBox>();
        theme1.set_based_on(Some(base_theme.clone()));
        theme1.add_setter(Setter::new(TemplatedControl::background_property(), Some(red.clone())));

        let theme2 = ControlTheme::for_type::<TextBox>();
        theme2.set_based_on(Some(base_theme));
        theme2.add_setter(Setter::new(TemplatedControl::background_property(), Some(green.clone())));

        let control = TextBox::new();
        control.set_theme(&theme1);
        leaf_panel.children().add(&control);

        let root = TestRoot::with_global_styles(true, Some(root_panel.upcast()));
        root.set_renderer(NullRenderer::new());

        root.layout_manager().execute_initial_layout_pass();

        Self { root, control, theme1, theme2, red: Some(red), green: Some(green), _app: app }
    }

    pub fn change_control_theme(&self) {
        if self.control.background() != self.red {
            panic!("Invalid benchmark state");
        }

        self.control.set_theme(&self.theme2);
        self.root.layout_manager().execute_layout_pass();

        if self.control.background() != self.green {
            panic!("Invalid benchmark state");
        }

        self.control.set_theme(&self.theme1);
        self.root.layout_manager().execute_layout_pass();

        if self.control.background() != self.red {
            panic!("Invalid benchmark state");
        }
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

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling", "ControlThemeChange");
    class.benchmark("change_control_theme", "", ControlThemeChange::new, |b| b.change_control_theme());
}

#[cfg(test)]
mod tests {
    #[test]
    fn control_theme_change() {
        crate::harness::smoke_class(super::register, "ControlThemeChange");
    }
}
