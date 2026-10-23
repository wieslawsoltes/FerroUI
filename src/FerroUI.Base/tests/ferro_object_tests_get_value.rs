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

#[test]
fn has_local_value_distinguishes_default_style_inheritance_and_explicit_default_values() {
    let parent = Class1::new();
    let target = Class2::with_parent(&parent);
    let property = Class1::baz_property();
    assert!(!target.has_local_value(property));
    parent.set_value(property, s("inherited"));
    assert_eq!(target.get_value(property), "inherited");
    assert!(!target.has_local_value(property));
    target.set_value_with_priority(property, s("style"), BindingPriority::Style);
    assert!(!target.has_local_value(property));
    target.set_current_value(property, s("overridden style"));
    assert!(!target.has_local_value(property));
    target.set_value(property, s("bazdefault"));
    assert!(target.has_local_value(property));
    target.clear_value(property);
    assert!(!target.has_local_value(property));
    assert_eq!(target.get_value(property), "style");
}

#[test]
fn has_local_value_ignores_animation_only_but_finds_local_values_hidden_by_animation() {
    let target = Class1::new();
    let property = Class1::foo_property();
    let animation = target.set_value_with_priority(property, s("animated"), BindingPriority::Animation).unwrap();
    assert!(!target.has_local_value(property));
    target.set_value(property, s("local"));
    assert_eq!(target.get_value(property), "animated");
    assert_eq!(target.get_base_value(property), Some(s("local")));
    assert!(target.has_local_value(property));
    animation.dispose();
    assert_eq!(target.get_value(property), "local");
    assert!(target.has_local_value(property));
}

#[test]
fn has_local_value_reports_binding_values_and_fallbacks_without_confusing_attachment_with_values() {
    let target = Class1::new();
    let property = Class1::foo_property();
    let source: Subject<BindingValue<String>> = Subject::new();
    let subscription = target.bind_value(property, source.observable(), BindingPriority::LocalValue);
    assert!(!target.has_local_value(property));
    source.on_next(BindingValue::new(s("bound")));
    assert!(target.has_local_value(property));
    source.on_next(BindingValue::unset());
    // The existing observable binding policy supplies the default as a local
    // fallback when the source produces Unset. Preserve that policy.
    assert_eq!(target.get_value(property), "foodefault");
    assert!(target.has_local_value(property));
    subscription.dispose();
    assert!(!target.has_local_value(property));
}
