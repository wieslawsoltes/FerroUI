//! Port of the upstream `BindingOperationsTests`.

use super::*;
use crate::data::converters::FuncMultiValueConverter;
use crate::data::core::{Value, ValueTypes};
use crate::data::model::Model;
use crate::data::{
    BindingBase, BindingOperations, BindingPriority, MultiBinding, ReflectionBinding, TemplateBinding,
};
use crate::layout::{Layoutable, LayoutableImpl};
use crate::styling::test_support::{set_child, TestRoot};
use crate::styling::{ControlTheme, Selectors, Setter, Style};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};

/// The equivalent of the upstream control class: an element with a tag.
#[repr(C)]
pub struct Control {
    base: Layoutable,
}

ferro_class!(Control: Layoutable);
ferro_impl_classes!(Control: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Control {
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Control, _>("Tag", None)
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct() })
    }
}

/// `new { Tag = "foo" }`.
struct TagData {
    tag: String,
}

ferro_model!(TagData, |b| b.read_only::<Value<String>>("Tag", |o| o.tag.clone()));

fn tag_data() -> Option<BoxedValue> {
    Some(Model::new_model(TagData { tag: s("foo") }))
}

fn tag_binding() -> Rc<dyn BindingBase> {
    ReflectionBinding::new("Tag")
}

#[test]
fn get_binding_expression_base_returns_null_when_not_bound() {
    let target = Control::new();
    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_none());
}

#[test]
fn get_binding_expression_base_returns_expression_when_bound() {
    for priority in [
        BindingPriority::Animation,
        BindingPriority::LocalValue,
        BindingPriority::Style,
        BindingPriority::StyleTrigger,
    ] {
        let data = tag_data();
        let target = Control::new();
        target.set_data_context(data);
        let binding = ReflectionBinding::new("Tag");
        binding.set_priority(priority);
        target.bind_binding(Control::tag_property(), &binding);

        let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
        assert!(expression.is_some(), "{priority:?}");
    }
}

#[test]
fn get_binding_expression_base_returns_expression_when_bound_locally_with_binding_error() {
    // The target has no data context so the binding fails.
    let target = Control::new();
    let binding = ReflectionBinding::new("Tag");
    target.bind_binding(Control::tag_property(), &binding);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}

#[test]
fn get_binding_expression_base_returns_expression_when_bound_to_multi_binding() {
    let data = tag_data();
    let target = Control::new();
    target.set_data_context(data);
    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(FuncMultiValueConverter::<Option<BoxedValue>, String>::new(|x| {
            x.iter().map(|v| ValueTypes::to_display_string(v.as_ref())).collect::<Vec<_>>().join(",")
        })))).with_bindings(vec![tag_binding(), tag_binding()]);
    target.bind_binding(Control::tag_property(), &binding);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}

#[test]
fn get_binding_expression_base_returns_binding_when_bound_via_control_theme() {
    let target = Control::new();
    let theme = ControlTheme::with_setters(
        Control::TYPE,
        [Setter::new_binding_base(Control::tag_property(), tag_binding())],
    );
    target.set_theme(theme);
    let _root = TestRoot::with_child(&target);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}

#[test]
fn get_binding_expression_base_returns_binding_when_bound_via_control_theme_template_binding() {
    let target = Control::new();
    let binding: Rc<dyn BindingBase> = TemplateBinding::new(Control::tag_property());
    let theme =
        ControlTheme::with_setters(Control::TYPE, [Setter::new_binding_base(Control::tag_property(), binding)]);
    target.set_theme(theme);
    let _root = TestRoot::with_child(&target);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}

#[test]
fn get_binding_expression_base_returns_binding_when_bound_via_control_theme_style() {
    let target = Control::new();
    target.classes().add("foo");
    let theme = ControlTheme::with_target_type(Control::TYPE);
    theme.add_style(Style::with_setters(
        Selectors::nesting(None).class("foo"),
        [Setter::new_binding_base(Control::tag_property(), tag_binding())],
    ));
    target.set_theme(theme);
    let _root = TestRoot::with_child(&target);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}

#[test]
fn get_binding_expression_base_returns_binding_when_bound_via_style() {
    let target = Control::new();
    let style = Style::with_setters(
        Selectors::of_type::<Control>(),
        [Setter::new_binding_base(Control::tag_property(), tag_binding())],
    );
    let root = TestRoot::new();
    root.styles().add(style);
    set_child(&root, &target);

    let expression = BindingOperations::get_binding_expression_base(&target, Control::tag_property());
    assert!(expression.is_some());
}
