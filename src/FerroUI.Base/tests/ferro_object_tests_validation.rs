//! Port of the upstream `Validation` object tests.
//!
//! Where upstream throws an argument exception, this port panics.

use super::*;
use crate::data::{BindingPriority, BindingValue};
use crate::*;

fn validate_foo(value: &i32) -> bool {
    *value < 100
}

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>("Qux", StyledPropertyOptions::new(11).validate(validate_foo))
    });

    ferro_property!(pub fn attached_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached_with::<Class1, Class1, _>(
            "Attached",
            StyledPropertyOptions::new(11).validate(validate_foo),
        )
    });
}

test_class!(Class2: FerroObject);

#[test]
fn registration_throws_if_default_value_fails_validation() {
    assert_panics(|| {
        StyledProperty::<i32>::create(
            "BadDefault",
            Class1::TYPE,
            Class1::TYPE,
            StyledPropertyMetadata::new(Some(101)),
            false,
            Some(Box::new(validate_foo)),
            None,
            false,
        );
    });
}

#[test]
fn metadata_override_throws_if_default_value_fails_validation() {
    assert_panics(|| Class1::foo_property().override_default_value::<Class2>(101));
}

#[test]
fn set_value_throws_if_fails_validation() {
    let target = Class1::new();

    assert_panics(|| target.set_value(Class1::foo_property(), 101));
}

#[test]
fn set_value_throws_if_fails_validation_attached() {
    let target = Class1::new();

    assert_panics(|| target.set_value(Class1::attached_property(), 101));
}

#[test]
fn reverts_to_default_value_if_local_value_binding_fails_validation() {
    let target = Class1::new();
    let source: Subject<i32> = Subject::new();

    target.bind(Class1::foo_property(), source.observable(), BindingPriority::LocalValue);
    source.on_next(150);

    assert_eq!(11, target.get_value(Class1::foo_property()));
}

#[test]
fn reverts_to_default_value_if_style_binding_fails_validation() {
    let target = Class1::new();
    let source: Subject<i32> = Subject::new();

    target.bind(Class1::foo_property(), source.observable(), BindingPriority::Style);
    source.on_next(150);

    assert_eq!(11, target.get_value(Class1::foo_property()));
}

#[test]
fn reverts_to_default_value_if_style_binding_fails_validation_2() {
    let target = Class1::new();
    let source: Subject<i32> = Subject::new();

    target.set_value_with_priority(Class1::foo_property(), 10, BindingPriority::Style);
    target.bind(Class1::foo_property(), source.observable(), BindingPriority::StyleTrigger);
    source.on_next(150);

    assert_eq!(11, target.get_value(Class1::foo_property()));
}

#[test]
fn reverts_to_default_value_if_style_binding_fails_validation_3() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let target = Class1::new();
        let source: Subject<BindingValue<i32>> = Subject::new();

        target.bind_value(Class1::foo_property(), source.observable(), priority);
        source.on_next(BindingValue::new(150));

        assert_eq!(11, target.get_value(Class1::foo_property()), "{priority:?}");
    }
}

#[test]
fn reverts_to_default_value_even_in_presence_of_other_bindings() {
    let target = Class1::new();
    let source1: Subject<i32> = Subject::new();
    let source2: Subject<i32> = Subject::new();

    target.bind(Class1::foo_property(), source1.observable(), BindingPriority::LocalValue);
    target.bind(Class1::foo_property(), source2.observable(), BindingPriority::LocalValue);
    source1.on_next(42);
    source2.on_next(150);

    assert_eq!(11, target.get_value(Class1::foo_property()));
}
