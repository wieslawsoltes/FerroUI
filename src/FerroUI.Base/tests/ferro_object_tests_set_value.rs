//! Port of the upstream `SetValue` object tests.

use super::*;
use crate::data::BindingPriority;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    // Upstream this is an `object` property. Untyped values hold exactly the
    // property's value type here, so it is a string property that is driven
    // through the untyped API in the tests that use it.
    ferro_property!(pub fn frank_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Frank", s("Kups"))
    });
}

test_class!(Class2: Class1);

impl Class2 {
    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class2, _>("Bar", s("bardefault"))
    });

    ferro_property!(pub fn flob_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class2, _>("Flob", 0.0)
    });

    ferro_property!(pub fn fred_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<Class2, _>("Fred", None)
    });
}

static_type!(AttachedOwner);

impl AttachedOwner {
    ferro_property!(pub fn attached_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<AttachedOwner, Class2, _>("Attached", None)
    });
}

#[test]
fn clear_value_clears_value() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));
    target.clear_value(Class1::foo_property());

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn clear_value_resets_value_to_style_value() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
    target.set_value(Class1::foo_property(), s("local"));

    assert_eq!("local", target.get_value(Class1::foo_property()));

    target.clear_value(Class1::foo_property());

    assert_eq!("style", target.get_value(Class1::foo_property()));
}

#[test]
fn clear_value_raises_property_changed() {
    let target = Class1::new();
    let raised = Counter::new();

    target.set_value(Class1::foo_property(), s("newvalue"));
    let (r, weak) = (raised.clone(), target.downgrade());
    target.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(BindingPriority::Unset, e.priority());
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(s("newvalue")), old_string(e));
        assert_eq!("foodefault", new_string(e));
        r.increment();
    });

    target.clear_value(Class1::foo_property());

    assert_eq!(1, raised.get());
}

#[test]
fn is_set_returns_false_for_unset_property() {
    let target = Class1::new();

    assert!(!target.is_set(Class1::foo_property()));
}

#[test]
fn is_set_returns_false_for_set_property() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("foo"));

    assert!(target.is_set(Class1::foo_property()));
}

#[test]
fn is_set_returns_false_for_cleared_property() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("foo"));
    target.set_value_untyped(Class1::foo_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert!(!target.is_set(Class1::foo_property()));
}

#[test]
fn set_value_sets_value() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));

    assert_eq!("newvalue", target.get_value(Class1::foo_property()));
}

#[test]
fn set_value_sets_attached_value() {
    let target = Class2::new();

    target.set_value(AttachedOwner::attached_property(), Some(s("newvalue")));

    assert_eq!(Some(s("newvalue")), target.get_value(AttachedOwner::attached_property()));
}

#[test]
fn set_value_raises_property_changed() {
    let target = Class1::new();
    let raised = Flag::new();

    let (r, weak) = (raised.clone(), target.downgrade());
    target.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::foo_property().as_property()
                && old_string(e) == Some(s("foodefault"))
                && new_string(e) == "newvalue",
        );
    });

    target.set_value(Class1::foo_property(), s("newvalue"));

    assert!(raised.get());
}

#[test]
fn set_value_style_priority_raises_property_changed() {
    let target = Class1::new();
    let raised = Flag::new();

    let (r, weak) = (raised.clone(), target.downgrade());
    target.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::foo_property().as_property()
                && old_string(e) == Some(s("foodefault"))
                && new_string(e) == "newvalue",
        );
    });

    target.set_value_with_priority(Class1::foo_property(), s("newvalue"), BindingPriority::Style);

    assert!(raised.get());
}

#[test]
fn set_value_doesnt_raise_property_changed_if_value_not_changed() {
    let target = Class1::new();
    let raised = Flag::new();

    target.set_value(Class1::foo_property(), s("bar"));

    let r = raised.clone();
    target.property_changed(move |_| r.set(true));

    target.set_value(Class1::foo_property(), s("bar"));

    assert!(!raised.get());
}

#[test]
fn set_value_doesnt_raise_property_changed_if_value_not_changed_from_default() {
    let target = Class1::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.property_changed(move |_| r.set(true));

    target.set_value(Class1::foo_property(), s("foodefault"));

    assert!(!raised.get());
}

#[test]
fn set_value_allows_setting_unregistered_property() {
    let target = Class1::new();

    assert!(!FerroPropertyRegistry::instance().is_registered_for(&target, Class2::bar_property()));

    target.set_value(Class2::bar_property(), s("bar"));

    assert_eq!("bar", target.get_value(Class2::bar_property()));
}

#[test]
fn set_value_allows_setting_unregistered_attached_property() {
    let target = Class1::new();

    assert!(!FerroPropertyRegistry::instance().is_registered_for(&target, AttachedOwner::attached_property()));

    target.set_value(AttachedOwner::attached_property(), Some(s("bar")));

    assert_eq!(Some(s("bar")), target.get_value(AttachedOwner::attached_property()));
}

#[test]
fn set_value_throws_exception_for_invalid_value_type() {
    let target = Class1::new();

    assert_panics(|| {
        target.set_value_untyped(Class1::foo_property(), &123, BindingPriority::LocalValue);
    });
}

#[test]
fn set_value_of_integer_on_double_property_works() {
    let target = Class2::new();

    target.set_value_untyped(Class2::flob_property(), &4, BindingPriority::LocalValue);

    let value = target.get_value(Class2::flob_property());
    assert_eq!(4.0, value);
}

/// The managed original finds the implicit operator of `ImplicitDouble` by
/// reflection; a type of the port states it with `ValueTypes::register_cast`.
#[test]
fn set_value_respects_implicit_conversions() {
    struct ImplicitDouble {
        value: f64,
    }
    crate::data::core::ValueTypes::register_cast::<ImplicitDouble, f64>(|v| v.value);

    let target = Class2::new();

    target.set_value_untyped(Class2::flob_property(), &ImplicitDouble { value: 4.0 }, BindingPriority::LocalValue);

    let value = target.get_value(Class2::flob_property());
    assert_eq!(4.0, value);
}

#[test]
fn set_value_can_convert_to_nullable() {
    let target = Class2::new();

    target.set_value_untyped(Class2::fred_property(), &4.0, BindingPriority::LocalValue);

    let value = target.get_value(Class2::fred_property());
    assert_eq!(Some(4.0), value);
}

#[test]
fn set_value_respects_priority() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("one"), BindingPriority::Template);
    assert_eq!("one", target.get_value(Class1::foo_property()));
    target.set_value_with_priority(Class1::foo_property(), s("two"), BindingPriority::Style);
    assert_eq!("one", target.get_value(Class1::foo_property()));
    target.set_value_with_priority(Class1::foo_property(), s("three"), BindingPriority::StyleTrigger);
    assert_eq!("three", target.get_value(Class1::foo_property()));
}

#[test]
fn set_value_style_doesnt_override_local_value() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("one"), BindingPriority::LocalValue);
    assert_eq!("one", target.get_value(Class1::foo_property()));
    target.set_value_with_priority(Class1::foo_property(), s("two"), BindingPriority::Style);
    assert_eq!("one", target.get_value(Class1::foo_property()));
}

#[test]
fn set_value_local_value_overrides_style() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("one"), BindingPriority::Style);
    assert_eq!("one", target.get_value(Class1::foo_property()));
    target.set_value_with_priority(Class1::foo_property(), s("two"), BindingPriority::LocalValue);
    assert_eq!("two", target.get_value(Class1::foo_property()));
}

#[test]
fn set_value_animation_overrides_local_value() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("one"), BindingPriority::LocalValue);
    assert_eq!("one", target.get_value(Class1::foo_property()));
    target.set_value_with_priority(Class1::foo_property(), s("two"), BindingPriority::Animation);
    assert_eq!("two", target.get_value(Class1::foo_property()));
}

#[test]
fn setting_unset_value_reverts_to_default_value() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));
    target.set_value_untyped(Class1::foo_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn setting_object_property_to_unset_value_reverts_to_default_value() {
    let target = Class1::new();

    target.set_value_untyped(Class1::frank_property(), &s("newvalue"), BindingPriority::LocalValue);
    target.set_value_untyped(Class1::frank_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert_eq!("Kups", target.get_value(Class1::frank_property()));
}

#[test]
fn setting_object_property_to_do_nothing_does_nothing() {
    let target = Class1::new();

    target.set_value_untyped(Class1::frank_property(), &s("newvalue"), BindingPriority::LocalValue);
    target.set_value_untyped(Class1::frank_property(), &DoNothingType, BindingPriority::LocalValue);

    assert_eq!("newvalue", target.get_value(Class1::frank_property()));
}

#[test]
fn disposing_style_set_value_reverts_to_default_value() {
    let target = Class1::new();

    let d = target.set_value_with_priority(Class1::foo_property(), s("foo"), BindingPriority::Style);
    d.expect("a disposable").dispose();

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn disposing_style_set_value_reverts_to_previous_style_value() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("foo"), BindingPriority::Style);
    let d = target.set_value_with_priority(Class1::foo_property(), s("bar"), BindingPriority::Style);
    d.expect("a disposable").dispose();

    assert_eq!("foo", target.get_value(Class1::foo_property()));
}

#[test]
fn disposing_animation_set_value_reverts_to_previous_local_value() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("foo"), BindingPriority::LocalValue);
    let d = target.set_value_with_priority(Class1::foo_property(), s("bar"), BindingPriority::Animation);
    d.expect("a disposable").dispose();

    assert_eq!("foo", target.get_value(Class1::foo_property()));
}
