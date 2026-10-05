//! Ported from the upstream `ExpressionObserverBuilderTests_Property`.

use super::test_support::*;
use ferroui_base::data::core::{BindingExpression, ModelRef, Value, ValueType};
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingChainException, BindingError, BindingErrorType, BindingNotification};
use ferroui_base::*;
use std::rc::Rc;

struct Inner {
    bar: i32,
}

ferro_model!(Inner, |b| b.read_only::<Value<i32>>("Bar", |o| o.bar));

struct Outer {
    foo: Rc<Inner>,
}

ferro_model!(Outer, |b| b.read_only::<ModelRef<Inner>>("Foo", |o| Some(o.foo.clone())));

fn data() -> Rc<Outer> {
    Model::new_model(Outer { foo: Model::new_model(Inner { bar: 1 }) })
}

fn build(source: &Rc<Outer>, path: &str) -> Rc<BindingExpression> {
    build_expression(Some(source.clone()), path, None, false).expect("a valid path")
}

#[test]
fn should_return_binding_notification_error_for_broken_chain() {
    let data = data();
    let target = build(&data, "Foo.Bar.Baz").to_observable(None);
    let result = take_one(&*target);

    let result = result.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()).expect("a notification");

    assert_eq!(
        result,
        &BindingNotification::with_error(
            BindingError::new(BindingChainException::with_expression(
                "Could not find a matching property accessor for 'Baz' on 'System.Int32'.".to_string(),
                "Foo.Bar.Baz",
                "Baz"
            )),
            BindingErrorType::Error
        )
    );
}

#[test]
fn should_have_null_source_type_for_broken_chain() {
    let data = data();
    let target = build(&data, "Foo.Bar.Baz");

    assert!(target.source_type().is_none());
}
