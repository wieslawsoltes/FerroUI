//! Ported from the upstream `TemplateBindingTests`.
//!
//! The upstream tests build the template of a button; here the template
//! child is created with its binding and then attached to the templated
//! parent in the order in which a templated control applies its template.

use super::test_support::*;
use ferroui_base::data::converters::{BoolConverters, IMultiValueConverter, IValueConverter, ObjectConverters};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::{BindingBase, BindingError, BindingMode, MultiBinding, TemplateBinding};
use ferroui_base::*;
use std::cell::RefCell;
use std::rc::Rc;

struct PrefixConverter;

fn to_text(value: &BoxedValue) -> String {
    ValueTypes::to_display_string(Some(value))
}

impl IValueConverter for PrefixConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        match (value, parameter) {
            (Some(value), Some(parameter)) => Ok(Some(boxed(to_text(parameter) + &to_text(value)))),
            _ => Ok(None),
        }
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        match (value, parameter) {
            (Some(value), Some(parameter)) => {
                let s = to_text(value);
                let prefix = to_text(parameter);
                match s.strip_prefix(&prefix) {
                    Some(rest) => Ok(Some(boxed(rest.to_string()))),
                    None => Ok(Some(boxed(s))),
                }
            }
            _ => Ok(None),
        }
    }
}

#[derive(Default)]
struct MultiConverter {
    values: RefCell<Vec<Option<BoxedValue>>>,
}

impl IMultiValueConverter for MultiConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.values.borrow_mut().extend(values.iter().cloned());
        Ok(values.first().cloned().flatten())
    }
}

/// Builds a button whose template is a content presenter with `binding` on
/// `property`, and applies the template.
fn build(
    source: &Ref<Button>,
    property: &'static FerroProperty,
    binding: &dyn BindingBase,
) -> Ref<ContentPresenter> {
    let target = ContentPresenter::new();
    target.bind_binding(property, binding);
    apply_template(source, &target);
    target
}

#[test]
fn one_way_binding_should_be_set_up() {
    let source = Button::new();
    let target = build(
        &source,
        ContentPresenter::content_property().as_property(),
        &TemplateBinding::new(ContentControl::content_property().as_property()),
    );

    assert!(target.content().is_none());
    source.set_content(bs("foo"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("foo"));
    source.set_content(bs("bar"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("bar"));
}

#[test]
fn two_way_binding_should_be_set_up() {
    let source = Button::new();
    let target = build(
        &source,
        ContentPresenter::content_property().as_property(),
        &TemplateBinding::new(ContentControl::content_property().as_property()).with_mode(BindingMode::TwoWay),
    );

    assert!(target.content().is_none());
    source.set_content(bs("foo"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("foo"));
    target.set_content(bs("bar"));
    assert_eq!(as_string(&source.content()).as_deref(), Some("bar"));
}

#[test]
fn converter_should_be_used() {
    let source = Button::new();
    let binding =
        TemplateBinding::new(ContentControl::content_property().as_property()).with_mode(BindingMode::TwoWay);
    binding.set_converter(Some(Rc::new(PrefixConverter)));
    binding.set_converter_parameter(bs("Hello "));
    let target = build(&source, ContentPresenter::content_property().as_property(), &binding);

    assert!(target.content().is_none());
    source.set_content(bs("foo"));
    assert_eq!(as_string(&target.content()).as_deref(), Some("Hello foo"));
    target.set_content(bs("Hello bar"));
    assert_eq!(as_string(&source.content()).as_deref(), Some("bar"));
}

#[test]
fn should_not_pass_unset_value_to_multi_binding_during_apply_template() {
    let converter = Rc::new(MultiConverter::default());
    let source = Button::new();
    source.set_content(bs("foo"));
    let binding = MultiBinding::new().with_converter_value(Some(converter.clone())).with_bindings(vec![TemplateBinding::new(ContentControl::content_property().as_property())]);
    let _target = build(&source, ContentPresenter::content_property().as_property(), &binding);

    // Issue #8672 was caused by the template binding passing "unset" to the
    // multi-binding while the template is applied, as the templated parent
    // property doesn't get set up until after the binding is initiated.
    let values: Vec<_> = converter.values.borrow().iter().map(as_string).collect();
    assert_eq!(values, vec![Some(s("foo"))]);
}

#[test]
fn should_execute_converter_without_specific_target_type() {
    // See issue #9766.
    let source = Button::new();
    let inner = TemplateBinding::new(ContentControl::content_property().as_property());
    inner.set_converter(Some(ObjectConverters::is_not_null()));
    let binding = MultiBinding::new().with_converter_value(Some(BoolConverters::and())).with_bindings(vec![inner]);
    let target = build(&source, Visual::is_visible_property().as_property(), &binding);

    assert!(!target.is_visible());
    source.set_content(bs("foo"));
    assert!(target.is_visible());
}

#[test]
fn can_bind_int_property_to_double() {
    let source = Button::new();
    source.set_opacity(42.0);
    let target = build(
        &source,
        ContentPresenter::max_lines_property().as_property(),
        &TemplateBinding::new(Visual::opacity_property().as_property()),
    );

    assert_eq!(target.max_lines(), 42);
}
