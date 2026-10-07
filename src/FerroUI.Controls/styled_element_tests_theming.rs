//! Port of the tests of `StyledElementTests_Theming.cs` (base unit tests)
//! that are not ported elsewhere; the others are in `ferroui-base`
//! (`styling/style_tests.rs`). The nested test classes of upstream
//! (`InlineTheme`, `ImplicitTheme`, `ThemeFromStyle`) are modules. Their
//! trees use templated controls, borders, canvases and buttons under a test
//! root, so these tests live with the controls.
//!
//! `ThemeFromStyle.Theme_Is_Applied_When_Attached_To_Logical_Tree` shares its
//! name with the `InlineTheme` test that is ported in `ferroui-base`; it is
//! ported here, in its module.

use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::templates::FuncControlTemplate;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Border, Button, Canvas, Control, ControlImpl, Panel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, CornerRadius, FerroObjectImpl, Ref, StyledElement,
    StyledElementImpl, Visual, VisualImpl,
};
use std::rc::Rc;

fn brush(brush: Rc<dyn IBrush>) -> Option<Rc<dyn IBrush>> {
    Some(brush)
}

fn tag_of(control: &Control) -> Option<String> {
    control.tag().as_ref().and_then(string_of)
}

fn create_theme(tag: &str) -> Ref<ControlTheme> {
    let template = FuncControlTemplate::for_type::<ThemedControl>(|_, _| {
        let border = Border::new();
        border.set_child(Border::new());
        border.upcast()
    });

    let result = ControlTheme::with_setters(
        ThemedControl::TYPE,
        [
            Setter::new(Control::tag_property(), boxed_str(tag)),
            Setter::new(TemplatedControl::template_property(), Some(template)),
            Setter::new(TemplatedControl::corner_radius_property(), CornerRadius::uniform(5.0)),
        ],
    );
    result.children().add(Style::with_setters(
        Selectors::nesting(None).template().of_type::<Border>(),
        [
            Setter::new(Border::background_property(), brush(Brushes::red())),
            Setter::new(Control::tag_property(), boxed_str(tag)),
        ],
    ));
    result.children().add(Style::with_setters(
        Selectors::nesting(None).class("foo").template().of_type::<Border>(),
        [Setter::new(Border::background_property(), brush(Brushes::green()))],
    ));
    result
}

fn create_derived_theme() -> Ref<ControlTheme> {
    let result = ControlTheme::with_setters(
        ThemedControl::TYPE,
        [Setter::new(Border::border_brush_property(), brush(Brushes::blue()))],
    );
    result.set_based_on(Some(create_theme("theme")));
    result.children().add(Style::with_setters(
        Selectors::nesting(None).template().of_type::<Border>(),
        [Setter::new(Border::border_brush_property(), brush(Brushes::yellow()))],
    ));
    result.children().add(Style::with_setters(
        Selectors::nesting(None).class("foo").template().of_type::<Border>(),
        [Setter::new(Border::border_brush_property(), brush(Brushes::cyan()))],
    ));
    result
}

fn single_or_default(target: &Visual) -> Option<Ref<Visual>> {
    let children = target.visual_children().to_vec();
    assert!(children.len() <= 1, "Sequence contains more than one element");
    children.into_iter().next()
}

#[repr(C)]
struct ThemedControl {
    base: TemplatedControl,
}

ferro_class!(ThemedControl: TemplatedControl);
ferro_impl_classes!(
    ThemedControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl ThemedControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TemplatedControl::construct() })
    }

    fn visual_child(&self) -> Option<Ref<Visual>> {
        single_or_default(self)
    }
}

#[repr(C)]
struct ThemedControl2 {
    base: TemplatedControl,
}

ferro_class!(ThemedControl2: TemplatedControl);
ferro_impl_classes!(
    ThemedControl2: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl ThemedControl2 {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TemplatedControl::construct() })
    }

    fn visual_child(&self) -> Option<Ref<Visual>> {
        single_or_default(self)
    }
}

mod inline_theme {
    use super::*;

    #[test]
    fn templated_parent_theme_is_detached_from_template_controls_when_theme_property_cleared() {
        let _scope = test_scope();
        let theme = ControlTheme::new();
        theme.set_target_type(Some(ThemedControl::TYPE));
        theme.children().add(Style::with_setters(
            Selectors::nesting(None).template().of_type::<Canvas>(),
            [Setter::new(Panel::background_property(), brush(Brushes::red()))],
        ));

        let target = create_target(Some(theme));
        target.set_template(Some(FuncControlTemplate::for_type::<ThemedControl>(|_, _| Canvas::new().upcast())));

        let _root = create_root(&target, false);

        let canvas = target.visual_child().and_then(|c| c.cast::<Canvas>()).expect("a Canvas");
        assert_eq!(brush(Brushes::red()), canvas.background());

        target.set_theme(None);

        assert_eq!(Some(canvas.clone().upcast::<Visual>()), target.visual_child());
        assert!(canvas.background().is_none());
    }

    #[test]
    fn primary_theme_is_not_detached_from_template_controls_when_theme_property_cleared() {
        let _scope = test_scope();
        let templated_parent_theme = ControlTheme::new();
        templated_parent_theme.set_target_type(Some(ThemedControl::TYPE));
        templated_parent_theme.children().add(Style::with_setters(
            Selectors::nesting(None).template().of_type::<Button>(),
            [Setter::new(Panel::background_property(), brush(Brushes::red()))],
        ));

        let child_theme = ControlTheme::with_setters(
            Button::TYPE,
            [Setter::new(TemplatedControl::foreground_property(), brush(Brushes::green()))],
        );

        let target = create_target(Some(templated_parent_theme));
        target.set_template(Some(FuncControlTemplate::for_type::<ThemedControl>(move |_, _| {
            let button = Button::new();
            button.set_theme(child_theme.clone());
            button.upcast()
        })));

        let _root = create_root(&target, true);

        let template_child = target.visual_child().and_then(|c| c.cast::<Button>()).expect("a Button");
        assert_eq!(brush(Brushes::red()), template_child.background());
        assert_eq!(brush(Brushes::green()), template_child.foreground());

        target.set_theme(None);

        assert!(template_child.background().is_none());
        assert_eq!(brush(Brushes::green()), template_child.foreground());
    }

    #[test]
    fn templated_parent_theme_is_not_detached_from_template_controls_when_primary_theme_property_cleared() {
        let _scope = test_scope();
        let templated_parent_theme = ControlTheme::new();
        templated_parent_theme.set_target_type(Some(ThemedControl::TYPE));
        templated_parent_theme.children().add(Style::with_setters(
            Selectors::nesting(None).template().of_type::<Button>(),
            [Setter::new(Panel::background_property(), brush(Brushes::red()))],
        ));

        let child_theme =
            ControlTheme::with_setters(Button::TYPE, [Setter::new(Control::tag_property(), boxed_str("childTheme"))]);

        let target = create_target(Some(templated_parent_theme));
        target.set_template(Some(FuncControlTemplate::for_type::<ThemedControl>(move |_, _| {
            let button = Button::new();
            button.set_theme(child_theme.clone());
            button.upcast()
        })));

        let _root = create_root(&target, true);

        let template_child = target.visual_child().and_then(|c| c.cast::<Button>()).expect("a Button");
        assert_eq!(brush(Brushes::red()), template_child.background());
        assert_eq!(Some("childTheme".to_string()), tag_of(&template_child));

        template_child.set_theme(None);

        assert_eq!(brush(Brushes::red()), template_child.background());
        assert!(template_child.tag().is_none());
    }

    #[test]
    fn unrelated_styles_are_not_detached_from_template_controls_when_theme_property_cleared() {
        let _scope = test_scope();
        let target = create_target(None);
        let _root = create_root(&target, true);

        let canvas = target.visual_child().and_then(|c| c.cast::<Border>()).expect("a Border");
        assert_eq!(Some("style".to_string()), tag_of(&canvas));

        target.set_theme(None);

        assert_eq!(Some(canvas.clone().upcast::<Visual>()), target.visual_child());
        assert_eq!(Some("style".to_string()), tag_of(&canvas));
    }

    fn create_target(theme: Option<Ref<ControlTheme>>) -> Ref<ThemedControl> {
        let result = ThemedControl::new();
        result.set_theme(theme.unwrap_or_else(|| create_theme("theme")));
        result
    }

    fn create_root(child: &Control, create_additional_styles: bool) -> Ref<TestRoot> {
        let result = TestRoot::new();

        if create_additional_styles {
            result.styles().add(Style::with_setters(
                Selectors::of_type::<ThemedControl>(),
                [Setter::new(Control::tag_property(), boxed_str("style"))],
            ));

            result.styles().add(Style::with_setters(
                Selectors::of_type::<Border>(),
                [Setter::new(Control::tag_property(), boxed_str("style"))],
            ));
        }

        result.set_child(child.to_ref());
        result.layout_manager().execute_initial_layout_pass();
        result
    }
}

mod implicit_theme {
    use super::*;

    #[test]
    fn nested_style_can_override_property_in_inner_templated_control() {
        let _scope = test_scope();
        let theme = ControlTheme::with_setters(
            ThemedControl2::TYPE,
            [Setter::new(
                TemplatedControl::template_property(),
                Some(FuncControlTemplate::for_type::<ThemedControl2>(|_, _| ThemedControl::new().upcast())),
            )],
        );
        theme.children().add(Style::with_setters(
            Selectors::nesting(None).template().of_type::<ThemedControl>(),
            [Setter::new(TemplatedControl::corner_radius_property(), CornerRadius::uniform(7.0))],
        ));
        let target = ThemedControl2::new();
        target.set_theme(theme);

        let _root = create_root(Some(&target), "theme");
        let inner = target.visual_child().and_then(|c| c.cast::<ThemedControl>()).expect("a ThemedControl");

        assert_eq!(CornerRadius::uniform(7.0), inner.corner_radius());
    }

    fn create_root(child: Option<&Control>, theme_tag: &str) -> Ref<TestRoot> {
        let result = TestRoot::new();
        result.resources().add(ThemedControl::TYPE, Some(Rc::new(create_theme(theme_tag))));
        result.set_child(child.map(Control::to_ref));
        result.layout_manager().execute_initial_layout_pass();
        result
    }
}

mod theme_from_style {
    use super::*;

    #[test]
    fn theme_is_applied_when_attached_to_logical_tree() {
        let _scope = test_scope();
        let target = create_target();

        assert!(target.theme().is_none());
        assert!(target.template().is_none());

        let _root = create_root(&target, None);

        assert!(target.theme().is_some());
        assert!(target.template().is_some());

        let border = target.visual_child().and_then(|c| c.cast::<Border>()).expect("a Border");
        assert_eq!(brush(Brushes::red()), border.background());

        target.classes().add("foo");
        assert_eq!(brush(Brushes::green()), border.background());
    }

    #[test]
    fn theme_can_be_set_to_local_value_while_updating_due_to_style_class() {
        let _scope = test_scope();
        let target = create_target();
        let theme1 = create_theme("theme");
        let theme2 = ControlTheme::with_target_type(ThemedControl::TYPE);
        let theme3 = ControlTheme::with_target_type(ThemedControl::TYPE);
        let root = TestRoot::new();
        root.styles().add(Style::with_setters(
            Selectors::of_type::<ThemedControl>(),
            [Setter::new(StyledElement::theme_property(), Some(theme1.clone()))],
        ));
        root.styles().add(Style::with_setters(
            Selectors::of_type::<ThemedControl>().class("bar"),
            [Setter::new(StyledElement::theme_property(), Some(theme2.clone()))],
        ));

        root.set_child(target.clone());
        root.layout_manager().execute_initial_layout_pass();

        assert_eq!(Some(theme1), target.theme());
        assert!(target.template().is_some());

        target.classes().add("bar");

        // At this point, theme2 has been promoted to a local value internally in StyledElement;
        // make sure that setting a new local value here doesn't cause it to be cleared when we
        // do a layout pass because StyledElement thinks its clearing the promoted theme.
        target.set_theme(theme3.clone());

        root.layout_manager().execute_layout_pass();

        assert_eq!(target.theme(), Some(theme3));
    }

    #[test]
    fn templated_parent_theme_change_applies_to_children() {
        let _scope = test_scope();
        let theme = create_derived_theme();
        let target = create_target();

        assert!(target.theme().is_none());
        assert!(target.template().is_none());

        let root = create_root(&target, theme.based_on());

        assert!(target.theme().is_some());
        assert!(target.template().is_some());

        root.styles().add(Style::with_setters(
            Selectors::of_type::<ThemedControl>().class("foo"),
            [Setter::new(StyledElement::theme_property(), Some(theme.clone()))],
        ));

        root.layout_manager().execute_layout_pass();

        let border = target.visual_child().and_then(|c| c.cast::<Border>()).expect("a Border");
        assert_eq!(brush(Brushes::red()), border.background());

        target.classes().add("foo");
        root.layout_manager().execute_layout_pass();

        assert_eq!(brush(Brushes::green()), border.background());
    }

    #[test]
    fn templated_parent_theme_change_applies_recursively_to_visual_children() {
        let _scope = test_scope();
        let theme = create_derived_theme();
        let target = create_target();

        assert!(target.theme().is_none());
        assert!(target.template().is_none());

        let root = create_root(&target, theme.based_on());

        assert!(target.theme().is_some());
        assert!(target.template().is_some());

        root.styles().add(Style::with_setters(
            Selectors::of_type::<ThemedControl>().class("foo"),
            [Setter::new(StyledElement::theme_property(), Some(theme.clone()))],
        ));

        root.layout_manager().execute_layout_pass();

        let border = target.visual_child().and_then(|c| c.cast::<Border>()).expect("a Border");
        let inner = border.child().and_then(|c| c.cast::<Border>()).expect("a Border");

        assert_eq!(brush(Brushes::red()), border.background());
        assert_eq!(brush(Brushes::red()), inner.background());

        assert_eq!(None, inner.border_brush());
        assert_eq!(None, inner.border_brush());

        target.classes().add("foo");
        root.layout_manager().execute_layout_pass();

        assert_eq!(brush(Brushes::green()), border.background());
        assert_eq!(brush(Brushes::green()), inner.background());

        assert_eq!(brush(Brushes::cyan()), inner.border_brush());
        assert_eq!(brush(Brushes::cyan()), inner.border_brush());
    }

    fn create_target() -> Ref<ThemedControl> {
        ThemedControl::new()
    }

    fn create_root(child: &Control, theme: Option<Ref<ControlTheme>>) -> Ref<TestRoot> {
        let result = TestRoot::new();
        result.styles().add(Style::with_setters(
            Selectors::of_type::<ThemedControl>(),
            [Setter::new(StyledElement::theme_property(), Some(theme.unwrap_or_else(|| create_theme("theme"))))],
        ));

        result.set_child(child.to_ref());
        result.layout_manager().execute_initial_layout_pass();
        result
    }
}
