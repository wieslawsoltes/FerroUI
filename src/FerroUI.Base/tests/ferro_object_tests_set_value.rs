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
fn untyped_markers_preserve_style_and_direct_priority_rules() {
    let target = Class1::new();
    let property = FerroProperty::register::<Class1, _>("UntypedMarkerPriority", 7i32);
    let style = target.set_value_with_priority(property, 11, BindingPriority::Style).unwrap();
    target.set_value(property, 23);

    for priority in [BindingPriority::Style, BindingPriority::Unset] {
        assert!(target.set_value_untyped(property, &UnsetValueType, priority).is_none());
        assert!(target.set_value_untyped(property, &DoNothingType, priority).is_none());
        assert_eq!(target.get_value(property), 23);
    }
    assert_panics(|| {
        target.set_value_untyped(property, &31i32, BindingPriority::Unset);
    });
    target.set_value_untyped(property, &UnsetValueType, BindingPriority::LocalValue);
    assert_eq!(target.get_value(property), 11);
    target.set_value(property, 41);
    target.set_current_value_untyped(property, &DoNothingType);
    assert_eq!(target.get_value(property), 41);
    target.set_current_value_untyped(property, &UnsetValueType);
    assert_eq!(target.get_value(property), 11);
    style.dispose();
    assert_eq!(target.get_value(property), 7);

    // This direct property's test storage is the existing string property.
    // Direct routes ignore the supplied priority, including Unset.
    let direct = FerroProperty::register_direct::<Class1, _>(
        "UntypedDirectMarkerPriority",
        |owner| owner.get_value(Class1::foo_property()).parse::<i32>().unwrap_or(0),
        Some(|owner, value: i32| owner.set_value(Class1::foo_property(), value.to_string())),
        -1,
    );
    target.set_value_untyped(direct, &53i32, BindingPriority::Unset);
    assert_eq!(target.get_direct_value(direct), 53);
    target.set_value_untyped(direct, &DoNothingType, BindingPriority::Unset);
    assert_eq!(target.get_direct_value(direct), 53);
    target.set_value_untyped(direct, &UnsetValueType, BindingPriority::Style);
    assert_eq!(target.get_direct_value(direct), -1);
    target.set_current_value_untyped(direct, &67i32);
    target.set_current_value_untyped(direct, &DoNothingType);
    assert_eq!(target.get_direct_value(direct), 67);
    target.set_current_value_untyped(direct, &UnsetValueType);
    assert_eq!(target.get_direct_value(direct), -1);
}

#[test]
fn untyped_conversion_temporary_drops_before_validation_or_coercion() {
    use crate::data::core::ValueTypes;

    struct Input;
    struct Output {
        number: i32,
        temporary: bool,
        dropped: Rc<Cell<bool>>,
        writes: Rc<Cell<usize>>,
    }
    impl Default for Output {
        fn default() -> Self {
            Self { number: 0, temporary: false, dropped: Rc::new(Cell::new(false)), writes: Rc::new(Cell::new(0)) }
        }
    }
    impl Clone for Output {
        fn clone(&self) -> Self {
            Self {
                number: self.number,
                temporary: false,
                dropped: self.dropped.clone(),
                writes: self.writes.clone(),
            }
        }
    }
    impl PartialEq for Output {
        fn eq(&self, other: &Self) -> bool {
            self.number == other.number
        }
    }
    impl Drop for Output {
        fn drop(&mut self) {
            if self.temporary {
                self.dropped.set(true);
            }
        }
    }

    let validations = Rc::new(Cell::new(0));
    let coercions = Rc::new(Cell::new(0));
    let validation_count = validations.clone();
    let coercion_count = coercions.clone();
    let styled = FerroProperty::register_with::<Class1, _>(
        "ConvertedTemporaryBeforeCallbacks",
        StyledPropertyOptions::new(Output::default())
            .validate(move |value| {
                if value.number == 29 {
                    assert!(value.dropped.get(), "conversion temporary survived into validation");
                    validation_count.set(validation_count.get() + 1);
                }
                true
            })
            .coerce(move |_, value| {
                if value.number == 29 {
                    assert!(value.dropped.get(), "conversion temporary survived into coercion");
                    coercion_count.set(coercion_count.get() + 1);
                }
                value
            }),
    );
    let direct = FerroProperty::register_direct::<Class1, _>(
        "ConvertedTemporaryBeforeDirectSetter",
        |_| Output::default(),
        Some(|_, value: Output| {
            assert!(value.dropped.get(), "conversion temporary survived into the direct setter");
            value.writes.set(value.writes.get() + 1);
        }),
        Output::default(),
    );

    for direct_route in [false, true] {
        for current in [false, true] {
            let dropped = Rc::new(Cell::new(false));
            let writes = Rc::new(Cell::new(0));
            let conversion_dropped = dropped.clone();
            let conversion_writes = writes.clone();
            ValueTypes::register_cast::<Input, Output>(move |_| Output {
                number: 29,
                temporary: true,
                dropped: conversion_dropped.clone(),
                writes: conversion_writes.clone(),
            });
            validations.set(0);
            coercions.set(0);
            let target = Class1::new();
            let property = if direct_route { direct.as_property() } else { styled.as_property() };

            if current {
                target.set_current_value_untyped(property, &Input);
            } else {
                target.set_value_untyped(property, &Input, BindingPriority::LocalValue);
            }

            assert!(dropped.get());
            if direct_route {
                assert_eq!(writes.get(), 1);
            } else {
                assert_eq!(target.get_value(styled).number, 29);
                // SetCurrentValue retains the original route's validation
                // bypass; both setter routes still apply coercion.
                if current {
                    assert_eq!(validations.get(), 0);
                } else {
                    assert!(validations.get() > 0);
                }
                assert!(coercions.get() > 0);
            }
        }
    }
}

#[test]
fn untyped_conversion_releases_its_temporary_when_value_clone_panics() {
    use crate::data::core::ValueTypes;

    struct Input;
    struct Output {
        temporary: bool,
        drops: Rc<Cell<usize>>,
        writes: Rc<Cell<usize>>,
    }
    impl Default for Output {
        fn default() -> Self {
            Self { temporary: false, drops: Rc::new(Cell::new(0)), writes: Rc::new(Cell::new(0)) }
        }
    }
    impl Clone for Output {
        fn clone(&self) -> Self {
            assert!(!self.temporary, "converted value clone failed");
            Self { temporary: false, drops: self.drops.clone(), writes: self.writes.clone() }
        }
    }
    impl PartialEq for Output {
        fn eq(&self, other: &Self) -> bool {
            self.temporary == other.temporary
        }
    }
    impl Drop for Output {
        fn drop(&mut self) {
            if self.temporary {
                self.drops.set(self.drops.get() + 1);
            }
        }
    }

    let styled = FerroProperty::register::<Class1, _>("PanickingConvertedClone", Output::default());
    let direct = FerroProperty::register_direct::<Class1, _>(
        "PanickingDirectConvertedClone",
        |_| Output::default(),
        Some(|_, value: Output| value.writes.set(value.writes.get() + 1)),
        Output::default(),
    );
    for direct_route in [false, true] {
        for current in [false, true] {
            let drops = Rc::new(Cell::new(0));
            let writes = Rc::new(Cell::new(0));
            let conversion_drops = drops.clone();
            let conversion_writes = writes.clone();
            ValueTypes::register_cast::<Input, Output>(move |_| Output {
                temporary: true,
                drops: conversion_drops.clone(),
                writes: conversion_writes.clone(),
            });
            let target = Class1::new();
            let property = if direct_route { direct.as_property() } else { styled.as_property() };

            assert_panics(|| {
                if current {
                    target.set_current_value_untyped(property, &Input);
                } else {
                    target.set_value_untyped(property, &Input, BindingPriority::LocalValue);
                }
            });

            assert_eq!(drops.get(), 1);
            assert_eq!(writes.get(), 0);
            assert!(!target.is_set(styled));
        }
    }
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
