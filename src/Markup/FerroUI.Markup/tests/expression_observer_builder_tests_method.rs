//! Ported from the upstream `ExpressionObserverBuilderTests_Method`.
//!
//! A method read through a binding is a delegate upstream; delegates have no
//! untyped equivalent in this port, where the method accessor produces a
//! command instead. The tests check the command where the upstream tests
//! check the delegate.

use super::test_support::*;
use ferroui_base::data::model::Model;
use ferroui_base::input::ICommand;
use ferroui_base::*;
use std::cell::Cell;
use std::rc::Rc;

struct TestObject {
    last_parameter: Cell<Option<i32>>,
}

impl TestObject {
    fn new() -> Rc<Self> {
        Model::new_model(Self { last_parameter: Cell::new(None) })
    }

    fn method_without_return(&self) {}

    fn method_with_return(&self) -> i32 {
        0
    }

    fn method_with_return_and_parameter(&self, i: Option<&BoxedValue>) -> i32 {
        let i = i.and_then(|i| i.downcast_ref::<i32>().copied()).expect("an integer");
        self.last_parameter.set(Some(i));
        i
    }

    fn static_method() {}
}

ferro_model!(TestObject, |b| b
    .method("MethodWithoutReturn", |o, _| o.method_without_return())
    .method("MethodWithReturn", |o, _| {
        o.method_with_return();
    })
    .method("MethodWithReturnAndParameter", |o, p| {
        o.method_with_return_and_parameter(p);
    })
    .method("StaticMethod", |_, _| TestObject::static_method()));

fn build(source: &Rc<TestObject>, path: &str) -> Option<BoxedValue> {
    let expression = build_expression(Some(source.clone()), path, None, false).expect("a valid path");
    take_one(&*expression.to_observable(None))
}

fn as_command(value: &Option<BoxedValue>) -> Option<Rc<dyn ICommand>> {
    value.as_ref().and_then(|v| v.downcast_ref::<Rc<dyn ICommand>>().cloned())
}

#[test]
fn should_get_method() {
    let data = TestObject::new();
    let result = build(&data, "MethodWithoutReturn");

    assert!(result.is_some());
}

/// The upstream theory checks the delegate type of each method; every
/// declared method is read as a command here.
#[test]
fn should_get_method_with_correct_delegate_type() {
    for method_name in ["MethodWithoutReturn", "MethodWithReturn", "MethodWithReturnAndParameter", "StaticMethod"] {
        let data = TestObject::new();
        let result = build(&data, method_name);

        assert!(as_command(&result).is_some(), "{method_name}");
    }
}

#[test]
fn can_call_method_returned_from_observer() {
    let data = TestObject::new();
    let result = build(&data, "MethodWithReturnAndParameter");

    let callback = as_command(&result).expect("a command");

    callback.execute(Some(&boxed(1)));
    assert_eq!(data.last_parameter.get(), Some(1));
}
