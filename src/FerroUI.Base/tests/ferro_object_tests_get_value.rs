//! Port of the upstream `GetValue` object tests.

use super::*;
use crate::data::BindingPriority;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>("Baz", StyledPropertyOptions::new(s("bazdefault")).inherits(true))
    });
}

#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class2 {
    fn class_init() {
        once_per_thread!({
            Class1::foo_property().override_default_value::<Class2>(s("foooverride"));
        });
    }

    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self { base: Class1::construct() })
    }

    pub fn with_parent(parent: &Ref<Class1>) -> Ref<Self> {
        let result = Self::new();
        result.set_inheritance_parent(parent);
        result
    }
}

test_class!(Class3: FerroObject);

#[test]
fn get_value_returns_default_value() {
    let target = Class1::new();

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn get_value_returns_overridden_default_value() {
    let target = Class2::new();

    assert_eq!("foooverride", target.get_value(Class1::foo_property()));
}

#[test]
fn get_value_returns_set_value() {
    let target = Class1::new();
    let property = Class1::foo_property();

    target.set_value(property, s("newvalue"));

    assert_eq!("newvalue", target.get_value(property));
}

#[test]
fn get_value_returns_bound_value() {
    let target = Class1::new();
    let property = Class1::foo_property();

    target.bind(property, Subject::behavior(s("newvalue")).observable(), BindingPriority::LocalValue);

    assert_eq!("newvalue", target.get_value(property));
}

#[test]
fn get_value_returns_inherited_value() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);

    parent.set_value(Class1::baz_property(), s("changed"));

    assert_eq!("changed", child.get_value(Class1::baz_property()));
}

#[test]
fn get_value_doesnt_throw_exception_for_unregistered_property() {
    let target = Class3::new();

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn get_base_value_ignores_default_value() {
    let target = Class3::new();

    target.set_value_with_priority(Class1::foo_property(), s("animated"), BindingPriority::Animation);
    assert!(target.get_base_value(Class1::foo_property()).is_none());
}

#[test]
fn get_base_value_returns_local_value() {
    let target = Class3::new();

    target.set_value(Class1::foo_property(), s("local"));
    target.set_value_with_priority(Class1::foo_property(), s("animated"), BindingPriority::Animation);
    assert_eq!(Some(s("local")), target.get_base_value(Class1::foo_property()));
}

#[test]
fn get_base_value_returns_style_value() {
    let target = Class3::new();

    target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
    target.set_value_with_priority(Class1::foo_property(), s("animated"), BindingPriority::Animation);
    assert_eq!(Some(s("style")), target.get_base_value(Class1::foo_property()));
}

#[test]
fn get_base_value_returns_style_value_set_via_untyped_setters() {
    let target = Class3::new();

    target.set_value_untyped(Class1::foo_property(), &s("style"), BindingPriority::Style);
    target.set_value_untyped(Class1::foo_property(), &s("animated"), BindingPriority::Animation);
    assert_eq!(Some(s("style")), target.get_base_value(Class1::foo_property()));
}
