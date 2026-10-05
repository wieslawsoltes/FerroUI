//! Ported from the upstream `MultiBindingTests`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{Maybe, Untyped, Value, ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{
    BindingBase, BindingError, BindingErrorType, BindingNotification, BindingOperations, MultiBinding,
};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Abc {
    a: i32,
    b: i32,
    c: i32,
}

impl Abc {
    fn new() -> Rc<Self> {
        Model::new_model(Self { a: 1, b: 2, c: 3 })
    }
}

ferro_model!(Abc, |b| b
    .read_only::<Value<i32>>("A", |o| o.a)
    .read_only::<Value<i32>>("B", |o| o.b)
    .read_only::<Value<i32>>("C", |o| o.c));

struct TestModel {
    non_notifying_value: Cell<Option<i32>>,
    notifying_value: RefCell<Option<BoxedValue>>,
    property_changed: Event<str>,
}

impl TestModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            non_notifying_value: Cell::new(Some(0)),
            notifying_value: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    fn set_notifying_value(&self, value: Option<BoxedValue>) {
        let changed = !ValueTypes::identity_equals(self.notifying_value.borrow().as_ref(), value.as_ref());
        if changed {
            self.notifying_value.replace(value);
            self.property_changed.raise("NotifyingValue");
        }
    }
}

impl INotifyPropertyChanged for TestModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestModel, |b| b
    .notify_property_changed()
    .property::<Maybe<i32>>("NonNotifyingValue", |o| o.non_notifying_value.get(), |o, v| o
        .non_notifying_value
        .set(v))
    .property::<Untyped>("NotifyingValue", |o| o.notifying_value.borrow().clone(), |o, v| o
        .set_notifying_value(v)));

#[derive(PartialEq)]
struct PlainObject;

/// The text of a value, as joining the values of a multi-binding shows it.
fn display(value: &Option<BoxedValue>) -> String {
    if BindingOperations::is_unset(value.as_ref()) {
        s("(unset)")
    } else if value.is_none() {
        String::new()
    } else {
        ValueTypes::to_display_string(value.as_ref())
    }
}

fn join(values: &[Option<BoxedValue>]) -> String {
    values.iter().map(display).collect::<Vec<_>>().join(",")
}

struct ConcatConverter;

impl IMultiValueConverter for ConcatConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(boxed(join(values))))
    }
}

struct UnsetValueConverter;

impl IMultiValueConverter for UnsetValueConverter {
    fn convert(
        &self,
        _values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(FerroProperty::unset_value()))
    }
}

struct NullValueConverter;

impl IMultiValueConverter for NullValueConverter {
    fn convert(
        &self,
        _values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(None)
    }
}

struct BindingNotificationConverter;

impl IMultiValueConverter for BindingNotificationConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(BindingNotification::with_error_and_fallback(
            BindingError::message("Value does not fall within the expected range."),
            BindingErrorType::Error,
            Some(boxed(join(values) + "-BindingNotification")),
        ))))
    }
}

struct TestModelMemberConverter;

impl IMultiValueConverter for TestModelMemberConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(model) = values[0].as_ref().and_then(|v| v.downcast_ref::<TestModel>()) else {
            return Ok(Some(boxed(String::new())));
        };
        Ok(Some(boxed(model.non_notifying_value.get().map(|v| v.to_string()).unwrap_or_default())))
    }
}

fn path(path: &str) -> Rc<dyn BindingBase> {
    Binding::with_path(path)
}

fn path_with_source(path: &str, source: &Rc<Abc>) -> Rc<dyn BindingBase> {
    let binding = Binding::with_path(path);
    binding.set_source(Some(source.clone()));
    binding
}

#[test]
fn one_way_binding_should_be_set_up() {
    let source = Abc::new();
    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(ConcatConverter))).with_bindings(vec![path("A"), path("B"), path("C")]);

    let target = Control::new();
    target.set_data_context(Some(source));
    target.bind_binding(Control::tag_property(), &binding);

    assert_eq!(as_string(&target.tag()).as_deref(), Some("1,2,3"));
}

#[test]
fn nested_multi_binding_should_be_set_up() {
    let source = Abc::new();
    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(ConcatConverter))).with_bindings(vec![
            path("A"),
            MultiBinding::new()
                .with_converter_value(Some(Rc::new(ConcatConverter)))
                .with_bindings(vec![path("B"), path("C")]),
        ]);

    let target = Control::new();
    target.set_data_context(Some(source));
    target.bind_binding(Control::tag_property(), &binding);

    assert_eq!(as_string(&target.tag()).as_deref(), Some("1,2,3"));
}

#[test]
fn should_return_fallback_value_when_converter_returns_unset_value() {
    let target = TextBlock::new();
    let _source = Abc::new();
    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(UnsetValueConverter))).with_bindings(vec![path("A"), path("B"), path("C")]).with_fallback_value(bs("fallback"));

    target.bind_binding(TextBlock::text_property(), &binding);

    assert!(target.text().is_some());
    assert_eq!(target.text().as_deref(), Some("fallback"));
}

#[test]
fn should_return_target_null_value_when_value_is_null() {
    let target = TextBlock::new();

    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(NullValueConverter))).with_bindings(vec![path("A"), path("B"), path("C")]).with_target_null_value(bs("(null)"));

    target.bind_binding(TextBlock::text_property(), &binding);

    assert!(target.text().is_some());
    assert_eq!(target.text().as_deref(), Some("(null)"));
}

#[test]
fn should_pass_unset_value_to_converter_for_broken_binding() {
    let source = Abc::new();
    let target = TextBlock::new();
    target.set_data_context(Some(source));

    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(ConcatConverter))).with_bindings(vec![path("A"), path("B"), path("Missing")]);

    target.bind_binding(TextBlock::text_property(), &binding);

    assert!(target.text().is_some());
    assert_eq!(target.text().as_deref(), Some("1,2,(unset)"));
}

#[test]
fn should_pass_fallback_value_to_converter_for_broken_binding() {
    let source = Abc::new();
    let target = TextBlock::new();
    target.set_data_context(Some(source));

    let missing = Binding::with_path("Missing");
    missing.set_fallback_value(bs("Fallback"));

    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(ConcatConverter))).with_bindings(vec![path("A"), path("B"), missing]);

    target.bind_binding(TextBlock::text_property(), &binding);

    assert!(target.text().is_some());
    assert_eq!(target.text().as_deref(), Some("1,2,Fallback"));
}

#[test]
fn multi_binding_without_string_format_and_converter() {
    let source = Abc::new();
    let target = ItemsControl::new();

    let binding = MultiBinding::new().with_bindings(vec![
            path_with_source("A", &source),
            path_with_source("B", &source),
            path_with_source("C", &source),
        ]);

    target.bind_binding(ItemsControl::items_source_property(), &binding);

    let items = target.items_source().expect("the values of the bindings");
    let items = items.downcast_ref::<Vec<Option<BoxedValue>>>().expect("a list of values");
    let item = |index: usize| items[index].as_ref().and_then(|v| v.downcast_ref::<i32>().copied());

    assert_eq!(items.len(), 3);
    assert_eq!(item(0), Some(source.a));
    assert_eq!(item(1), Some(source.b));
    assert_eq!(item(2), Some(source.c));
}

#[test]
fn converter_can_return_binding_notification() {
    let source = Abc::new();
    let target = TextBlock::new();
    target.set_data_context(Some(source));

    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(BindingNotificationConverter))).with_bindings(vec![path("A"), path("B"), path("C")]);

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("1,2,3-BindingNotification"));
}

#[test]
fn converter_should_be_called_on_property_changed_even_if_property_not_changed() {
    // Issue #16084
    let data = TestModel::new();
    let target = TextBlock::new();
    target.set_data_context(Some(data.clone()));

    let binding = MultiBinding::new().with_converter_value(Some(Rc::new(TestModelMemberConverter))).with_bindings(vec![Binding::new(), path("NotifyingValue")]);

    target.bind_binding(TextBlock::text_property(), &binding);
    assert_eq!(target.text().as_deref(), Some("0"));

    data.non_notifying_value.set(Some(1));
    assert_eq!(target.text().as_deref(), Some("0"));

    data.set_notifying_value(Some(Rc::new(PlainObject)));
    assert_eq!(target.text().as_deref(), Some("1"));
}
