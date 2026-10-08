//! Port of the observable-based parts of the upstream `Binding` object tests.
//!
//! Not ported: tests that need the binding classes (`Binding`, two-way
//! bindings to view models) or the dispatcher.

use super::*;
use crate::data::{BindingPriority, BindingValue};
use crate::reactive::{IObservable, Observable, ObservableExt};
use crate::*;
use std::rc::Rc;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn qux_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class1, _>("Qux", 5.6)
    });

    ferro_property!(pub fn double_value_property() -> StyledProperty<f64> {
        FerroProperty::register::<Class1, _>("DoubleValue", 0.0)
    });

    // Not upstream: a nullable reference-typed property, for the test that
    // binds a null value.
    ferro_property!(pub fn nullable_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<Class1, _>("Nullable", Some(s("nullabledefault")))
    });
}

test_class!(Class2: Class1);

impl Class2 {
    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class2, _>("Bar", s("bardefault"))
    });
}

type Source = Subject<BindingValue<String>>;

/// Returns an observable that returns a single value but does not complete.
fn single(value: &str) -> Rc<dyn IObservable<BindingValue<String>>> {
    Observable::single_value(bv(value))
}

fn untyped<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<BoxedValue>> {
    source.select(|v| boxed(v))
}

const LOCAL: BindingPriority = BindingPriority::LocalValue;

#[test]
fn bind_sets_current_value() {
    let target = Class1::new();
    let source = Source::behavior(bv("initial"));
    let property = Class1::foo_property();

    target.bind_value(property, source.observable(), LOCAL);

    assert_eq!("initial", target.get_value(property));
}

#[test]
fn bind_raises_property_changed() {
    let target = Class1::new();
    let source = Source::new();
    let raised = Counter::new();

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(s("foodefault")), old_string(e));
        assert_eq!("newvalue", new_string(e));
        assert_eq!(BindingPriority::LocalValue, e.priority());
        r.increment();
    });

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("newvalue"));

    assert_eq!(1, raised.get());
}

#[test]
fn property_changed_not_raised_when_value_unchanged() {
    let target = Class1::new();
    let source = Source::new();
    let raised = Counter::new();

    let r = raised.clone();
    target.property_changed(move |_| r.increment());
    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("newvalue"));
    source.on_next(bv("newvalue"));

    assert_eq!(1, raised.get());
}

#[test]
fn setting_local_value_overrides_binding_until_binding_produces_next_value() {
    let target = Class1::new();
    let source = Source::new();
    let property = Class1::foo_property();

    target.bind_value(property, source.observable(), LOCAL);
    source.on_next(bv("foo"));
    assert_eq!("foo", target.get_value(property));

    target.set_value(property, s("bar"));
    assert_eq!("bar", target.get_value(property));

    source.on_next(bv("baz"));
    assert_eq!("baz", target.get_value(property));
}

#[test]
fn completing_local_value_binding_reverts_to_default_value_even_when_local_value_set_earlier() {
    let target = Class1::new();
    let source = Source::new();
    let property = Class1::foo_property();

    target.bind_value(property, source.observable(), LOCAL);
    source.on_next(bv("foo"));
    target.set_value(property, s("bar"));
    source.on_next(bv("baz"));
    source.on_completed();

    assert_eq!("foodefault", target.get_value(property));
}

#[test]
fn disposing_local_value_binding_should_not_revert_to_set_local_value() {
    let target = Class1::new();
    let source = Source::behavior(bv("bar"));

    target.set_value(Class1::foo_property(), s("foo"));
    let sub = target.bind_value(Class1::foo_property(), source.observable(), LOCAL);

    assert_eq!("bar", target.get_value(Class1::foo_property()));

    sub.dispose();

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn local_value_binding_should_override_style_binding() {
    let target = Class1::new();
    let source1 = Source::behavior(bv("foo"));
    let source2 = Source::behavior(bv("bar"));

    target.bind_value(Class1::foo_property(), source1.observable(), BindingPriority::Style);

    assert_eq!("foo", target.get_value(Class1::foo_property()));

    target.bind_value(Class1::foo_property(), source2.observable(), BindingPriority::LocalValue);

    assert_eq!("bar", target.get_value(Class1::foo_property()));
}

#[test]
fn style_binding_should_not_override_local_value_binding() {
    let target = Class1::new();
    let source1 = Source::behavior(bv("foo"));
    let source2 = Source::behavior(bv("bar"));

    target.bind_value(Class1::foo_property(), source1.observable(), BindingPriority::LocalValue);

    assert_eq!("foo", target.get_value(Class1::foo_property()));

    target.bind_value(Class1::foo_property(), source2.observable(), BindingPriority::Style);

    assert_eq!("foo", target.get_value(Class1::foo_property()));
}

#[test]
fn completing_animation_binding_reverts_to_set_local_value() {
    let target = Class1::new();
    let source = Source::new();
    let property = Class1::foo_property();

    target.set_value(property, s("foo"));
    target.bind_value(property, source.observable(), BindingPriority::Animation);
    source.on_next(bv("bar"));
    source.on_completed();

    assert_eq!("foo", target.get_value(property));
}

#[test]
fn completing_animation_binding_reverts_to_set_local_value_with_style_value() {
    let target = Class1::new();
    let source = Source::new();
    let property = Class1::foo_property();

    target.set_value_with_priority(property, s("style"), BindingPriority::Style);
    target.set_value(property, s("foo"));
    target.bind_value(property, source.observable(), BindingPriority::Animation);
    source.on_next(bv("bar"));
    source.on_completed();

    assert_eq!("foo", target.get_value(property));
}

#[test]
fn completing_local_value_binding_raises_property_changed() {
    let target = Class1::new();
    let source = Source::behavior(bv("foo"));
    let property = Class1::foo_property();
    let raised = Counter::new();

    target.bind_value(property, source.observable(), LOCAL);
    assert_eq!("foo", target.get_value(property));

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(BindingPriority::Unset, e.priority());
        assert_eq!(property.as_property(), e.property());
        assert_eq!(Some(s("foo")), old_string(e));
        assert_eq!("foodefault", new_string(e));
        r.increment();
    });

    source.on_completed();

    assert_eq!("foodefault", target.get_value(property));
    assert_eq!(1, raised.get());
}

#[test]
fn completing_style_binding_raises_property_changed() {
    let target = Class1::new();
    let source = Source::behavior(bv("foo"));
    let property = Class1::foo_property();
    let raised = Counter::new();

    target.bind_value(property, source.observable(), BindingPriority::Style);
    assert_eq!("foo", target.get_value(property));

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(BindingPriority::Unset, e.priority());
        assert_eq!(property.as_property(), e.property());
        assert_eq!(Some(s("foo")), old_string(e));
        assert_eq!("foodefault", new_string(e));
        r.increment();
    });

    source.on_completed();

    assert_eq!("foodefault", target.get_value(property));
    assert_eq!(1, raised.get());
}

#[test]
fn completing_local_value_binding_with_style_binding_raises_property_changed() {
    let target = Class1::new();
    let source = Source::behavior(bv("foo"));
    let property = Class1::foo_property();
    let raised = Counter::new();

    target.bind_value(property, Source::behavior(bv("bar")).observable(), BindingPriority::Style);
    target.bind_value(property, source.observable(), LOCAL);
    assert_eq!("foo", target.get_value(property));

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(BindingPriority::Style, e.priority());
        assert_eq!(property.as_property(), e.property());
        assert_eq!(Some(s("foo")), old_string(e));
        assert_eq!("bar", new_string(e));
        r.increment();
    });

    source.on_completed();

    assert_eq!("bar", target.get_value(property));
    assert_eq!(1, raised.get());
}

#[test]
fn disposing_local_value_binding_raises_property_changed() {
    let target = Class1::new();
    let source = Source::behavior(bv("foo"));
    let property = Class1::foo_property();
    let raised = Counter::new();

    let sub = target.bind_value(property, source.observable(), LOCAL);
    assert_eq!("foo", target.get_value(property));

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(BindingPriority::Unset, e.priority());
        assert_eq!(property.as_property(), e.property());
        assert_eq!(Some(s("foo")), old_string(e));
        assert_eq!("foodefault", new_string(e));
        r.increment();
    });

    sub.dispose();

    assert_eq!("foodefault", target.get_value(property));
    assert_eq!(1, raised.get());
}

#[test]
fn setting_style_value_overrides_binding_permanently() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    target.bind(Class1::foo_property(), source.observable(), BindingPriority::Style);
    source.on_next(s("foo"));
    assert_eq!("foo", target.get_value(Class1::foo_property()));

    target.set_value_with_priority(Class1::foo_property(), s("bar"), BindingPriority::Style);
    assert_eq!("bar", target.get_value(Class1::foo_property()));

    source.on_next(s("baz"));
    assert_eq!("bar", target.get_value(Class1::foo_property()));
}

#[test]
fn second_local_value_binding_unsubscribes_first() {
    let property = Class1::foo_property();
    let target = Class1::new();
    let source1 = Source::new();
    let source2 = Source::new();

    target.bind_value(property, source1.observable(), BindingPriority::LocalValue);
    target.bind_value(property, source2.observable(), BindingPriority::LocalValue);

    source1.on_next(bv("foo"));
    assert_eq!("foodefault", target.get_value(property));

    source2.on_next(bv("bar"));
    assert_eq!("bar", target.get_value(property));

    source1.on_next(bv("baz"));
    assert_eq!("bar", target.get_value(property));
}

#[test]
fn completing_second_local_value_binding_doesnt_revert_to_first() {
    let property = Class1::foo_property();
    let target = Class1::new();
    let source1 = Source::new();
    let source2 = Source::new();

    target.bind_value(property, source1.observable(), BindingPriority::LocalValue);
    target.bind_value(property, source2.observable(), BindingPriority::LocalValue);

    source1.on_next(bv("foo"));
    source2.on_next(bv("bar"));
    source1.on_next(bv("baz"));
    source2.on_completed();

    assert_eq!("foodefault", target.get_value(property));
}

#[test]
fn completing_style_trigger_binding_reverts_to_style_binding() {
    let property = Class1::foo_property();
    let target = Class1::new();
    let source1 = Source::new();
    let source2 = Source::new();

    target.bind_value(property, source1.observable(), BindingPriority::Style);
    target.bind_value(property, source2.observable(), BindingPriority::StyleTrigger);

    source1.on_next(bv("foo"));
    source2.on_next(bv("bar"));
    source2.on_completed();
    source1.on_next(bv("baz"));

    assert_eq!("baz", target.get_value(property));
}

#[test]
fn bind_non_generic_sets_current_value() {
    let target = Class1::new();
    let source = Class1::new();

    source.set_value(Class1::foo_property(), s("initial"));
    target.bind_property_untyped(
        Class1::foo_property(),
        untyped(source.get_observable(Class1::foo_property())),
        LOCAL,
    );

    assert_eq!("initial", target.get_value(Class1::foo_property()));
}

// Upstream binds `null` to a string property. Strings are not nullable here,
// so this uses a property of an optional string.
#[test]
fn bind_non_generic_can_set_null_on_reference_type() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::behavior(boxed(None::<String>));
    let property = Class1::nullable_property();

    target.bind_untyped(property, source.observable(), LOCAL);

    assert_eq!(None, target.get_value(property));
}

#[test]
fn local_value_bind_generic_to_value_type_accepts_unset_value() {
    let target = Class1::new();
    let source: Subject<BindingValue<f64>> = Subject::new();

    target.bind_value(Class1::qux_property(), source.observable(), LOCAL);
    source.on_next(BindingValue::new(6.7));
    source.on_next(BindingValue::unset());

    assert_eq!(5.6, target.get_value(Class1::qux_property()));
    assert!(target.is_set(Class1::qux_property()));
}

#[test]
fn local_value_bind_non_generic_to_value_type_accepts_unset_value() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), LOCAL);
    source.on_next(boxed(6.7));
    source.on_next(FerroProperty::unset_value());

    assert_eq!(5.6, target.get_value(Class1::qux_property()));
    assert!(target.is_set(Class1::qux_property()));
}

#[test]
fn style_bind_non_generic_to_value_type_accepts_unset_value() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), BindingPriority::Style);
    source.on_next(boxed(6.7));
    source.on_next(FerroProperty::unset_value());

    assert_eq!(5.6, target.get_value(Class1::qux_property()));
    assert!(target.is_set(Class1::qux_property()));
}

#[test]
fn local_value_bind_non_generic_to_value_type_accepts_do_nothing() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), LOCAL);
    source.on_next(boxed(6.7));
    source.on_next(boxed(DoNothingType));

    assert_eq!(6.7, target.get_value(Class1::qux_property()));
}

#[test]
fn style_bind_non_generic_to_value_type_accepts_do_nothing() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), BindingPriority::Style);
    source.on_next(boxed(6.7));
    source.on_next(boxed(DoNothingType));

    assert_eq!(6.7, target.get_value(Class1::qux_property()));
}

#[test]
fn one_time_binding_ignores_unset_value() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), LOCAL);

    source.on_next(FerroProperty::unset_value());
    assert_eq!(5.6, target.get_value(Class1::qux_property()));

    source.on_next(boxed(6.7));
    assert_eq!(6.7, target.get_value(Class1::qux_property()));
}

// Upstream produces a binding notification object; the equivalent untyped
// value here is a boxed binding value in the error state.
#[test]
fn one_time_binding_ignores_binding_errors() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    target.bind_untyped(Class1::qux_property(), source.observable(), LOCAL);

    source.on_next(boxed(BindingValue::<f64>::binding_error(error("error"))));
    assert_eq!(5.6, target.get_value(Class1::qux_property()));

    source.on_next(boxed(6.7));
    assert_eq!(6.7, target.get_value(Class1::qux_property()));
}

#[test]
fn bind_does_not_throw_exception_for_unregistered_property() {
    let target = Class1::new();

    target.bind_value(Class2::bar_property(), single("foo"), LOCAL);

    assert_eq!("foo", target.get_value(Class2::bar_property()));
}

#[test]
fn bind_sets_subsequent_value() {
    let target = Class1::new();
    let source = Class1::new();

    source.set_value(Class1::foo_property(), s("initial"));
    target.bind(Class1::foo_property(), source.get_observable(Class1::foo_property()), LOCAL);
    source.set_value(Class1::foo_property(), s("subsequent"));

    assert_eq!("subsequent", target.get_value(Class1::foo_property()));
}

#[test]
fn bind_ignores_invalid_value_type() {
    let target = Class1::new();
    target.bind_property_untyped(Class1::foo_property(), Observable::return_(boxed(123)), LOCAL);
    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn observable_is_unsubscribed_when_subscription_disposed() {
    let source = Source::test(bv("foo"));
    let target = Class1::new();

    let subscription = target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    assert_eq!(1, source.subscriber_count());

    subscription.dispose();
    assert_eq!(0, source.subscriber_count());
}

#[test]
fn observable_is_unsubscribed_when_new_binding_of_same_priority_is_added() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation] {
        let source1 = Source::test(bv("foo"));
        let source2 = Source::test(bv("bar"));
        let target = Class1::new();

        target.bind_value(Class1::foo_property(), source1.observable(), priority);
        assert_eq!(1, source1.subscriber_count(), "{priority:?}");

        target.bind_value(Class1::foo_property(), source2.observable(), priority);
        assert_eq!(1, source2.subscriber_count(), "{priority:?}");
        assert_eq!(0, source1.subscriber_count(), "{priority:?}");
    }
}

#[test]
fn observable_is_unsubscribed_when_new_binding_of_higher_priority_is_added() {
    let source1 = Source::test(bv("foo"));
    let source2 = Source::test(bv("bar"));
    let target = Class1::new();

    target.bind_value(Class1::foo_property(), source1.observable(), BindingPriority::Style);
    assert_eq!(1, source1.subscriber_count());

    target.bind_value(Class1::foo_property(), source2.observable(), BindingPriority::Template);
    assert_eq!(1, source2.subscriber_count());
    assert_eq!(0, source1.subscriber_count());
}

#[test]
fn observable_is_unsubscribed_when_new_value_of_same_priority_is_added() {
    for priority in [BindingPriority::Style, BindingPriority::Animation] {
        let source = Source::test(bv("foo"));
        let target = Class1::new();

        target.bind_value(Class1::foo_property(), source.observable(), priority);
        assert_eq!(1, source.subscriber_count(), "{priority:?}");

        target.set_value_with_priority(Class1::foo_property(), s("foo"), priority);
        assert_eq!(0, source.subscriber_count(), "{priority:?}");
    }
}

#[test]
fn observable_is_unsubscribed_when_new_value_of_higher_priority_is_added() {
    let source = Source::test(bv("foo"));
    let target = Class1::new();

    target.bind_value(Class1::foo_property(), source.observable(), BindingPriority::Style);
    assert_eq!(1, source.subscriber_count());

    target.set_value_with_priority(Class1::foo_property(), s("foo"), BindingPriority::Template);
    assert_eq!(0, source.subscriber_count());
}

#[test]
fn observable_is_not_unsubscribed_when_animation_value_is_set() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let source = Source::test(bv("foo"));
        let target = Class1::new();

        target.bind_value(Class1::foo_property(), source.observable(), priority);
        assert_eq!(1, source.subscriber_count(), "{priority:?}");

        target.set_value_with_priority(Class1::foo_property(), s("bar"), BindingPriority::Animation);
        assert_eq!(1, source.subscriber_count(), "{priority:?}");
    }
}

#[test]
fn observable_is_not_unsubscribed_when_animation_binding_is_added() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let source1 = Source::test(bv("foo"));
        let source2 = Source::test(bv("bar"));
        let target = Class1::new();

        target.bind_value(Class1::foo_property(), source1.observable(), priority);
        assert_eq!(1, source1.subscriber_count(), "{priority:?}");

        target.bind_value(Class1::foo_property(), source2.observable(), BindingPriority::Animation);
        assert_eq!(1, source1.subscriber_count(), "{priority:?}");
        assert_eq!(1, source2.subscriber_count(), "{priority:?}");
    }
}

#[test]
fn local_value_binding_is_not_unsubscribed_when_local_value_is_set() {
    let source = Source::test(bv("foo"));
    let target = Class1::new();

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    assert_eq!(1, source.subscriber_count());

    target.set_value(Class1::foo_property(), s("foo"));
    assert_eq!(1, source.subscriber_count());
}

#[test]
fn two_way_separate_binding_works() {
    let obj1 = Class1::new();
    let obj2 = Class1::new();

    obj1.set_value(Class1::foo_property(), s("initial1"));
    obj2.set_value(Class1::foo_property(), s("initial2"));

    obj1.bind(Class1::foo_property(), obj2.get_observable(Class1::foo_property()), LOCAL);
    obj2.bind(Class1::foo_property(), obj1.get_observable(Class1::foo_property()), LOCAL);

    assert_eq!("initial2", obj1.get_value(Class1::foo_property()));
    assert_eq!("initial2", obj2.get_value(Class1::foo_property()));

    obj1.set_value(Class1::foo_property(), s("first"));

    assert_eq!("first", obj1.get_value(Class1::foo_property()));
    assert_eq!("first", obj2.get_value(Class1::foo_property()));

    obj2.set_value(Class1::foo_property(), s("second"));

    assert_eq!("second", obj1.get_value(Class1::foo_property()));
    assert_eq!("second", obj2.get_value(Class1::foo_property()));

    obj1.set_value(Class1::foo_property(), s("third"));

    assert_eq!("third", obj1.get_value(Class1::foo_property()));
    assert_eq!("third", obj2.get_value(Class1::foo_property()));
}

#[test]
fn two_way_binding_with_priority_works() {
    let obj1 = Class1::new();
    let obj2 = Class1::new();
    let style = BindingPriority::Style;

    obj1.set_value_with_priority(Class1::foo_property(), s("initial1"), style);
    obj2.set_value_with_priority(Class1::foo_property(), s("initial2"), style);

    obj1.bind(Class1::foo_property(), obj2.get_observable(Class1::foo_property()), style);
    obj2.bind(Class1::foo_property(), obj1.get_observable(Class1::foo_property()), style);

    assert_eq!("initial2", obj1.get_value(Class1::foo_property()));
    assert_eq!("initial2", obj2.get_value(Class1::foo_property()));

    obj1.set_value_with_priority(Class1::foo_property(), s("first"), style);

    assert_eq!("first", obj1.get_value(Class1::foo_property()));
    assert_eq!("first", obj2.get_value(Class1::foo_property()));

    obj2.set_value_with_priority(Class1::foo_property(), s("second"), style);

    assert_eq!("first", obj1.get_value(Class1::foo_property()));
    assert_eq!("second", obj2.get_value(Class1::foo_property()));

    obj1.set_value_with_priority(Class1::foo_property(), s("third"), style);

    assert_eq!("third", obj1.get_value(Class1::foo_property()));
    assert_eq!("second", obj2.get_value(Class1::foo_property()));
}

#[test]
fn local_binding_overwrites_local_value() {
    let target = Class1::new();
    let binding = Source::new();

    target.bind_value(Class1::foo_property(), binding.observable(), LOCAL);

    binding.on_next(bv("first"));
    assert_eq!("first", target.get_value(Class1::foo_property()));

    target.set_value(Class1::foo_property(), s("second"));
    assert_eq!("second", target.get_value(Class1::foo_property()));

    binding.on_next(bv("third"));
    assert_eq!("third", target.get_value(Class1::foo_property()));
}

#[test]
fn style_binding_overrides_default_value() {
    let target = Class1::new();

    target.bind_value(Class1::foo_property(), single("stylevalue"), BindingPriority::Style);

    assert_eq!("stylevalue", target.get_value(Class1::foo_property()));
}

// Upstream `this_Operator_Returns_Value_Property`: the indexer is the untyped
// getter.
#[test]
fn untyped_getter_returns_value_property() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));

    let value = target.get_value_untyped(Class1::foo_property());
    assert_eq!(Some(&s("newvalue")), value.downcast_ref::<String>());
}

// Upstream `this_Operator_Sets_Value_Property`: the indexer is the untyped
// setter.
#[test]
fn untyped_setter_sets_value_property() {
    let target = Class1::new();

    target.set_value_untyped(Class1::foo_property(), &s("newvalue"), LOCAL);

    assert_eq!("newvalue", target.get_value(Class1::foo_property()));
}

// Upstream `this_Operator_Doesnt_Accept_Observable`.
#[test]
fn untyped_setter_doesnt_accept_observable() {
    let target = Class1::new();

    assert_panics(|| {
        let observable: Rc<dyn IObservable<String>> = Observable::return_(s("newvalue"));
        target.set_value_untyped(Class1::foo_property(), &observable, LOCAL);
    });
}

#[test]
fn binding_error_reverts_to_default_value() {
    let target = Class1::new();
    let source = Source::new();

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("initial"));
    source.on_next(BindingValue::binding_error(error("Foo")));

    assert_eq!("foodefault", target.get_value(Class1::foo_property()));
}

#[test]
fn binding_error_with_fallback_value_causes_target_update() {
    let target = Class1::new();
    let source = Source::new();

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("initial"));
    source.on_next(BindingValue::binding_error_with_fallback(error("Foo"), Some(s("bar"))));

    assert_eq!("bar", target.get_value(Class1::foo_property()));
}

#[test]
fn data_validation_error_does_not_cause_target_update() {
    let target = Class1::new();
    let source = Source::new();

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("initial"));
    source.on_next(BindingValue::data_validation_error(error("Foo")));

    assert_eq!("initial", target.get_value(Class1::foo_property()));
}

#[test]
fn data_validation_error_with_fallback_value_causes_target_update() {
    let target = Class1::new();
    let source = Source::new();

    target.bind_value(Class1::foo_property(), source.observable(), LOCAL);
    source.on_next(bv("initial"));
    source.on_next(BindingValue::data_validation_error_with_fallback(error("Foo"), Some(s("bar"))));

    assert_eq!("bar", target.get_value(Class1::foo_property()));
}

#[test]
fn is_animating_on_property_with_no_value_returns_false() {
    let target = Class1::new();

    assert!(!target.is_animating(Class1::foo_property()));
}

#[test]
fn is_animating_on_property_with_animation_value_returns_true() {
    let target = Class1::new();
    let source = Source::behavior(bv("foo"));

    target.bind_value(Class1::foo_property(), source.observable(), BindingPriority::Animation);

    assert!(target.is_animating(Class1::foo_property()));
}

#[test]
fn is_animating_on_property_with_non_animation_binding_returns_false() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    target.bind(Class1::foo_property(), source.observable(), BindingPriority::LocalValue);

    assert!(!target.is_animating(Class1::foo_property()));
}

#[test]
fn is_animating_on_property_with_animation_binding_returns_true() {
    let target = Class1::new();
    let source = Subject::behavior(s("foo"));

    target.bind(Class1::foo_property(), source.observable(), BindingPriority::Animation);

    assert!(target.is_animating(Class1::foo_property()));
}

#[test]
fn is_animating_on_property_with_local_value_and_animation_binding_returns_true() {
    let target = Class1::new();
    let source = Subject::behavior(s("foo"));

    target.set_value(Class1::foo_property(), s("bar"));
    target.bind(Class1::foo_property(), source.observable(), BindingPriority::Animation);

    assert!(target.is_animating(Class1::foo_property()));
}

#[test]
fn is_animating_returns_true_when_animated_value_is_same_as_local_value() {
    let target = Class1::new();
    let source = Subject::behavior(s("foo"));

    target.set_value(Class1::foo_property(), s("foo"));
    target.bind(Class1::foo_property(), source.observable(), BindingPriority::Animation);

    assert!(target.is_animating(Class1::foo_property()));
}

#[test]
fn disposing_completed_binding_does_not_throw() {
    let target = Class1::new();
    let source = Source::new();
    let subscription = target.bind_value(Class1::foo_property(), source.observable(), LOCAL);

    source.on_completed();

    subscription.dispose();
}

#[test]
fn binding_producing_unset_value_does_not_cause_unsubscribe() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let target = Class1::new();
        let source = Source::new();

        target.bind_value(Class1::foo_property(), source.observable(), priority);

        source.on_next(bv("foo"));
        assert_eq!("foo", target.get_value(Class1::foo_property()), "{priority:?}");
        source.on_next(BindingValue::unset());
        assert_eq!("foodefault", target.get_value(Class1::foo_property()), "{priority:?}");
        source.on_next(bv("bar"));
        assert_eq!("bar", target.get_value(Class1::foo_property()), "{priority:?}");
    }
}

#[test]
fn produces_correct_values_and_base_values_with_multiple_animation_bindings() {
    let target = Class1::new();
    let source1: Subject<BindingValue<f64>> = Subject::behavior(BindingValue::new(12.2));
    let source2: Subject<BindingValue<f64>> = Subject::behavior(BindingValue::new(13.3));

    target.set_value(Class1::qux_property(), 11.1);
    target.bind_value(Class1::qux_property(), source1.observable(), BindingPriority::Animation);

    assert_eq!(12.2, target.get_value(Class1::qux_property()));
    assert_eq!(Some(11.1), target.get_base_value(Class1::qux_property()));

    target.bind_value(Class1::qux_property(), source2.observable(), BindingPriority::Animation);

    assert_eq!(13.3, target.get_value(Class1::qux_property()));
    assert_eq!(Some(11.1), target.get_base_value(Class1::qux_property()));

    source2.on_completed();

    assert_eq!(12.2, target.get_value(Class1::qux_property()));
    assert_eq!(Some(11.1), target.get_base_value(Class1::qux_property()));

    source1.on_completed();

    assert_eq!(11.1, target.get_value(Class1::qux_property()));
    assert_eq!(Some(11.1), target.get_base_value(Class1::qux_property()));
}

// --- binding indexer ----------------------------------------------------------
//
// Upstream `target[!Property]` reads as `target.indexer(&Property.bind())`,
// `target[!Property] = binding` as `target.bind_indexer(&Property.bind(), &binding)`
// and the two-way `!!Property` as `!Property.bind()`.

#[test]
fn this_operator_binds_one_way() {
    let target1 = Class1::new();
    let target2 = Class2::new();
    let binding = Class2::bar_property().bind().with_mode(crate::data::BindingMode::OneWay);

    target1.set_value(Class1::foo_property(), s("first"));
    target2.bind_indexer(&binding, &target1.indexer(&Class1::foo_property().bind()));
    target1.set_value(Class1::foo_property(), s("second"));

    assert_eq!("second", target2.get_value(Class2::bar_property()));
}

#[test]
fn this_operator_binds_two_way() {
    let target1 = Class1::new();
    let target2 = Class1::new();

    target1.set_value(Class1::foo_property(), s("first"));
    target2.bind_indexer(&Class1::foo_property().bind(), &target1.indexer(&!Class1::foo_property().bind()));
    assert_eq!("first", target2.get_value(Class1::foo_property()));
    target1.set_value(Class1::foo_property(), s("second"));
    assert_eq!("second", target2.get_value(Class1::foo_property()));
    target2.set_value(Class1::foo_property(), s("third"));
    assert_eq!("third", target1.get_value(Class1::foo_property()));
}

#[test]
fn this_operator_binds_one_time() {
    let target1 = Class1::new();
    let target2 = Class1::new();

    target1.set_value(Class1::foo_property(), s("first"));
    target2.bind_indexer(
        &Class1::foo_property().bind(),
        &target1.indexer(&Class1::foo_property().bind().with_mode(crate::data::BindingMode::OneTime)),
    );
    target1.set_value(Class1::foo_property(), s("second"));

    assert_eq!("first", target2.get_value(Class1::foo_property()));
}

/// Not from upstream, which covers `ToBinding` through the setter and
/// multi-binding tests: an observable bound as a binding delivers its
/// values to the property.
#[test]
fn to_binding_binds_values_of_observable() {
    let target = Class1::new();
    let source = Subject::<String>::behavior(s("initial"));
    let property = Class1::foo_property();

    let binding = to_binding(source.observable());
    let _expression = target.bind_binding(property.as_property(), &*binding);

    assert_eq!("initial", target.get_value(property));

    source.on_next(s("second"));

    assert_eq!("second", target.get_value(property));
}
