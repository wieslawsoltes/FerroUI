//! Ported from the upstream `ExpressionObserverBuilderTests_Negation`.

use super::test_support::*;
use ferroui_base::data::core::{BindingExpression, Untyped, UntypedBindingExpression, Value, ValueType};
use ferroui_base::data::model::{Event, INotifyDataErrorInfo, Model};
use ferroui_base::data::{
    BindingChainException, BindingError, BindingErrorType, BindingNotification, DataValidationException,
};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A model with a single read-only property `Foo` holding any value.
struct FooModel {
    foo: Option<BoxedValue>,
}

impl FooModel {
    fn new<T: PartialEq + 'static>(foo: T) -> Rc<Self> {
        Model::new_model(Self { foo: Some(boxed(foo)) })
    }
}

ferro_model!(FooModel, |b| b.read_only::<Untyped>("Foo", |o| o.foo.clone()));

/// A value that does not convert to a boolean.
#[derive(PartialEq)]
struct PlainObject;

/// A value without any declared property.
#[derive(PartialEq)]
struct Empty;

struct Test {
    data_validation_error: RefCell<Option<String>>,
    foo: Cell<bool>,
    bar: RefCell<Option<BoxedValue>>,
    errors_changed: Event<str>,
}

impl Test {
    fn new(data_validation_error: &str) -> Rc<Self> {
        Model::new_model(Self {
            data_validation_error: RefCell::new(Some(s(data_validation_error))),
            foo: Cell::new(false),
            bar: RefCell::new(None),
            errors_changed: Event::new(),
        })
    }
}

impl INotifyDataErrorInfo for Test {
    fn has_errors(&self) -> bool {
        self.data_validation_error.borrow().as_ref().is_some_and(|e| !e.trim().is_empty())
    }

    fn get_errors(&self, _property_name: Option<&str>) -> Vec<BoxedValue> {
        match &*self.data_validation_error.borrow() {
            Some(error) => vec![boxed(error.clone())],
            None => Vec::new(),
        }
    }

    fn errors_changed(&self) -> &Event<str> {
        &self.errors_changed
    }
}

ferro_model!(Test, |b| b
    .notify_data_error_info()
    .property::<Value<bool>>("Foo", |o| o.foo.get(), |o, v| o.foo.set(v))
    .property::<Untyped>("Bar", |o| o.bar.borrow().clone(), |o, v| {
        o.bar.replace(v);
    }));

/// The source is held weakly by the expression: the caller keeps it alive.
fn build<T: AnyValue>(source: &Rc<T>, path: &str, enable_data_validation: bool) -> Rc<BindingExpression> {
    build_expression(Some(source.clone()), path, None, enable_data_validation).expect("a valid path")
}

fn build_and_take<T: AnyValue>(source: &Rc<T>, path: &str, enable_data_validation: bool) -> Option<BoxedValue> {
    take_one(&*build(source, path, enable_data_validation).to_observable(None))
}

fn as_bool(value: &Option<BoxedValue>) -> Option<bool> {
    value.as_ref().and_then(|v| v.downcast_ref::<bool>().copied())
}

#[track_caller]
fn assert_notification(result: &Option<BoxedValue>, expected: BindingNotification) {
    let result = result.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()).expect("a notification");
    assert_eq!(result, &expected);
}

fn chain_error(message: String, expression: &str, error_point: &str) -> BindingNotification {
    BindingNotification::with_error(
        BindingError::new(BindingChainException::with_expression(message, expression, error_point)),
        BindingErrorType::Error,
    )
}

#[test]
fn should_negate_0() {
    let data = FooModel::new(0);
    let result = build_and_take(&data, "!Foo", false);

    assert_eq!(as_bool(&result), Some(true));
}

#[test]
fn should_negate_1() {
    let data = FooModel::new(1);
    let result = build_and_take(&data, "!Foo", false);

    assert_eq!(as_bool(&result), Some(false));
}

#[test]
fn should_negate_false_string() {
    let data = FooModel::new(s("false"));
    let result = build_and_take(&data, "!Foo", false);

    assert_eq!(as_bool(&result), Some(true));
}

#[test]
fn should_negate_true_string() {
    let data = FooModel::new(s("True"));
    let result = build_and_take(&data, "!Foo", false);

    assert_eq!(as_bool(&result), Some(false));
}

#[test]
fn should_return_binding_notification_for_string_not_convertible_to_boolean() {
    let data = FooModel::new(s("foo"));
    let result = build_and_take(&data, "!Foo", false);

    assert_notification(&result, chain_error(s("Unable to convert 'foo' to bool."), "!Foo", "!"));
}

#[test]
fn should_return_binding_notification_for_value_not_convertible_to_boolean() {
    let data = FooModel::new(PlainObject);
    let result = build_and_take(&data, "!Foo", false);

    assert_notification(
        &result,
        chain_error(
            format!("Unable to convert '{}' to bool.", ValueType::of::<PlainObject>().name()),
            "!Foo",
            "!",
        ),
    );
}

#[test]
fn should_negate_binding_notification_value() {
    let data = FooModel::new(true);
    let result = build_and_take(&data, "!Foo", true);

    assert_notification(&result, BindingNotification::new(Some(boxed(false))));
}

#[test]
fn should_pass_through_binding_notification_error() {
    let data = Rc::new(Empty);
    let result = build_and_take(&data, "!Foo", true);

    assert_notification(
        &result,
        chain_error(
            format!(
                "Could not find a matching property accessor for 'Foo' on '{}'.",
                ValueType::of::<Empty>().name()
            ),
            "!Foo",
            "Foo",
        ),
    );
}

#[test]
fn should_negate_binding_notification_error_fallback_value() {
    let data = Test::new("Test error");
    let result = build_and_take(&data, "!Foo", true);

    assert_notification(
        &result,
        BindingNotification::with_error_and_fallback(
            BindingError::new(DataValidationException::new(Some(boxed(s("Test error"))))),
            BindingErrorType::DataValidationError,
            Some(boxed(true)),
        ),
    );
}

#[test]
fn set_value_should_return_false_for_invalid_value() {
    let data = FooModel::new(s("foo"));
    let target = build(&data, "!Foo", false);
    let (_values, _sub) = record(&*target.to_observable(None));

    assert!(!target.write_value_to_source(bs("bar")));
}
