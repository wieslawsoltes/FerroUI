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

// The indexer of the reference class is the untyped
// getter.
#[test]
fn this_operator_returns_value_property() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));

    let value = target.get_value_untyped(Class1::foo_property());
    assert_eq!(Some(&s("newvalue")), value.downcast_ref::<String>());
}

// The indexer of the reference class is the untyped
// setter.
#[test]
fn this_operator_sets_value_property() {
    let target = Class1::new();

    target.set_value_untyped(Class1::foo_property(), &s("newvalue"), LOCAL);

    assert_eq!("newvalue", target.get_value(Class1::foo_property()));
}

#[test]
fn this_operator_doesnt_accept_observable() {
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

#[test]
fn boxed_frame_disposes_the_subscription_returned_after_synchronous_completion_or_error() {
    for fail in [false, true] {
        let target = Class1::new();
        let property = Class1::qux_property();
        let disposed = Counter::new();
        let changes = Recorder::new();
        let log = changes.clone();
        target.property_changed(move |change| log.push(change.get_new_value::<f64>()));
        let count = disposed.clone();
        let source = Observable::create(move |observer| {
            observer.on_next(boxed(12.5));
            if fail {
                observer.on_error(Rc::new(std::io::Error::other("source failed")));
            } else {
                observer.on_completed();
            }
            let count = count.clone();
            Disposable::create(move || count.increment())
        });

        let binding = target.bind_property_untyped(property, source, BindingPriority::Style);
        assert_eq!(changes.get(), vec![12.5, 5.6]);
        assert_eq!(target.get_value(property), 5.6);
        assert_eq!(disposed.get(), 1);
        binding.dispose();
        assert_eq!(disposed.get(), 1);
    }
}

#[test]
fn boxed_frame_subscribes_when_unmasked_and_restores_default_after_disposal() {
    let target = Class1::new();
    let property = Class1::qux_property();
    let source = Subject::behavior(boxed(8.5));
    target.set_value(property, 15.0);
    let binding = target.bind_property_untyped(property, source.observable(), BindingPriority::Style);
    assert_eq!(source.subscriber_count(), 0);

    target.clear_value(property);
    assert_eq!(source.subscriber_count(), 1);
    assert_eq!(target.get_value(property), 8.5);
    source.on_next(boxed(BindingValue::<f64>::data_validation_error(error("invalid"))));
    assert_eq!(target.get_value(property), 8.5);
    source.on_next(boxed(BindingValue::<f64>::binding_error_with_fallback(error("fallback"), Some(9.5))));
    assert_eq!(target.get_value(property), 9.5);

    binding.dispose();
    assert_eq!(source.subscriber_count(), 0);
    source.on_next(boxed(21.0));
    assert_eq!(target.get_value(property), 5.6);
}

#[test]
fn boxed_bindings_snapshot_values_instead_of_retaining_the_incoming_box() {
    #[derive(PartialEq)]
    struct Snapshot(Cell<i32>);

    impl Clone for Snapshot {
        fn clone(&self) -> Self {
            Self(Cell::new(self.0.get()))
        }
    }

    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let target = Class1::new();
        let property = FerroProperty::register::<Class1, _>("BoxedSnapshot", Snapshot(Cell::new(0)));
        let source = Subject::<BoxedValue>::new();
        let binding = target.bind_untyped(property, source.observable(), priority);
        let input = Rc::new(Snapshot(Cell::new(17)));

        source.on_next(input.clone());
        input.0.set(29);

        assert_eq!(target.get_value(property).0.get(), 17);
        assert_eq!(Rc::strong_count(&input), 1);
        binding.dispose();
    }
}

#[test]
fn retained_boxed_source_observer_does_not_keep_a_disposed_bindings_values_alive() {
    #[derive(Clone, PartialEq)]
    struct Payload(Option<Rc<()>>);

    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let target = Class1::new();
        let property = FerroProperty::register::<Class1, _>("WeakBoxedObserver", Payload(None));
        let retained = Rc::new(RefCell::new(None::<Rc<dyn IObserver<BoxedValue>>>));
        let saved = retained.clone();
        let source = Observable::create(move |observer| {
            *saved.borrow_mut() = Some(observer);
            Disposable::empty()
        });
        let binding = target.bind_untyped(property, source, priority);
        let weak_binding = Rc::downgrade(&binding);
        let token = Rc::new(());
        let weak_token = Rc::downgrade(&token);
        retained.borrow().as_ref().unwrap().on_next(boxed(Payload(Some(token))));
        assert!(weak_token.upgrade().is_some());

        binding.dispose();
        drop(binding);
        // A local binding leaves its last effective value in the store.
        target.clear_value(property);

        assert!(weak_binding.upgrade().is_none());
        assert!(weak_token.upgrade().is_none());
        assert!(retained.borrow().is_some());
        retained.borrow().as_ref().unwrap().on_next(boxed(Payload(Some(Rc::new(())))));
        assert!(target.get_value(property).0.is_none());
    }
}

#[test]
fn boxed_frame_keeps_its_first_fallback_across_unsubscribe_and_resubscribe() {
    // The property is declared by Class1. Class2 initially inherits that
    // metadata, so its first override below is a valid metadata transition.
    let target = Class2::new();
    let property = FerroProperty::register::<Class1, _>("CachedBoxedFallback", 11i32);
    let source = Subject::behavior(FerroProperty::unset_value());
    let binding = target.bind_untyped(property, source.observable(), BindingPriority::Style);
    assert_eq!(target.get_value(property), 11);
    assert_eq!(source.subscriber_count(), 1);

    target.set_value(property, 29);
    assert_eq!(source.subscriber_count(), 0);
    property.override_metadata::<Class2>(StyledPropertyMetadata::new(Some(47)));
    target.clear_value(property);

    assert_eq!(source.subscriber_count(), 1);
    assert_eq!(target.get_value(property), 11);
    binding.dispose();
    assert_eq!(source.subscriber_count(), 0);
    assert_eq!(target.get_value(property), 47);

    let replacement = target.bind_untyped(property, source.observable(), BindingPriority::Style);
    assert_eq!(target.get_value(property), 47);
    replacement.dispose();
}

#[test]
fn boxed_notification_keeps_snapshots_and_drops_them_after_payload_clone() {
    use crate::data::{BindingError, BindingErrorType, BindingNotification};
    use std::fmt;

    struct Payload {
        number: i32,
        on_clone: Option<Rc<dyn Fn()>>,
        on_drop: Option<Rc<dyn Fn()>>,
    }
    impl Clone for Payload {
        fn clone(&self) -> Self {
            if let Some(on_clone) = &self.on_clone {
                on_clone();
            }
            Self { number: self.number, on_clone: None, on_drop: None }
        }
    }
    impl PartialEq for Payload {
        fn eq(&self, other: &Self) -> bool {
            self.number == other.number
        }
    }
    impl Drop for Payload {
        fn drop(&mut self) {
            if let Some(on_drop) = self.on_drop.take() {
                on_drop();
            }
        }
    }
    #[derive(Debug)]
    struct ErrorProbe(Rc<RefCell<Vec<&'static str>>>);
    impl fmt::Display for ErrorProbe {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("snapshot error")
        }
    }
    impl std::error::Error for ErrorProbe {}
    impl Drop for ErrorProbe {
        fn drop(&mut self) {
            self.0.borrow_mut().push("error drop");
        }
    }

    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let validation_log = log.clone();
        let property = FerroProperty::register_with::<Class1, _>(
            "NotificationSnapshotLifetime",
            StyledPropertyOptions::new(Payload { number: 0, on_clone: None, on_drop: None }).validate(move |v| {
                if v.number == 7 {
                    assert_eq!(&validation_log.borrow()[..3], &["clone", "payload drop", "error drop"]);
                    validation_log.borrow_mut().push("validate");
                }
                true
            }),
        );
        let target = Class1::new();
        let source = Subject::<BoxedValue>::new();
        let binding = target.bind_untyped(property, source.observable(), priority);
        let notification = Rc::new(BindingNotification::with_error(
            BindingError::new(ErrorProbe(log.clone())),
            BindingErrorType::Error,
        ));
        // The error still has to be snapshotted, although this notification
        // will produce an ordinary value and release that snapshot locally.
        notification.set_error_type(BindingErrorType::None);
        let weak = Rc::downgrade(&notification);
        let clone_log = log.clone();
        let drop_log = log.clone();
        notification.set_value(Some(boxed(Payload {
            number: 7,
            on_clone: Some(Rc::new(move || {
                clone_log.borrow_mut().push("clone");
                let notification = weak.upgrade().unwrap();
                notification.set_value(None);
                notification.set_error(None);
                // Reentrant mutation must not release either original
                // snapshot until copying the original payload completes.
                assert_eq!(&*clone_log.borrow(), &["clone"]);
            })),
            on_drop: Some(Rc::new(move || drop_log.borrow_mut().push("payload drop"))),
        })));

        source.on_next(notification.clone());

        assert_eq!(target.get_value(property).number, 7);
        assert!(notification.value().is_none());
        assert!(notification.error().is_none());
        assert_eq!(&log.borrow()[..3], &["clone", "payload drop", "error drop"]);
        assert!(log.borrow()[3..].contains(&"validate"));
        binding.dispose();
    }
}

#[test]
fn boxed_binding_conversion_releases_its_temporary_before_delivery_or_clone_unwind() {
    use crate::data::core::ValueTypes;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[derive(PartialEq)]
    struct Input;
    struct Output {
        number: i32,
        temporary: bool,
        panic_on_clone: bool,
        dropped: Rc<Cell<bool>>,
    }
    impl Clone for Output {
        fn clone(&self) -> Self {
            assert!(!self.panic_on_clone, "converted value clone failed");
            Self { number: self.number, temporary: false, panic_on_clone: false, dropped: self.dropped.clone() }
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

    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        for panic_on_clone in [false, true] {
            let dropped = Rc::new(Cell::new(false));
            let validations = Rc::new(Cell::new(0));
            let observed_validations = validations.clone();
            let property = FerroProperty::register_with::<Class1, _>(
                "BindingConversionTemporary",
                StyledPropertyOptions::new(Output {
                    number: 0,
                    temporary: false,
                    panic_on_clone: false,
                    dropped: dropped.clone(),
                }).validate(move |value| {
                    if value.number == 23 {
                        assert!(value.dropped.get(), "conversion temporary survived into validation");
                        observed_validations.set(observed_validations.get() + 1);
                    }
                    true
                }),
            );
            let conversion_dropped = dropped.clone();
            ValueTypes::register_cast::<Input, Output>(move |_| Output {
                number: 23,
                temporary: true,
                panic_on_clone,
                dropped: conversion_dropped.clone(),
            });
            let target = Class1::new();
            let retained = Rc::new(RefCell::new(None::<Rc<dyn IObserver<BoxedValue>>>));
            let saved = retained.clone();
            let source = Observable::create(move |observer| {
                *saved.borrow_mut() = Some(observer);
                Disposable::empty()
            });
            let binding = target.bind_untyped(property, source, priority);
            let observer = retained.borrow().as_ref().unwrap().clone();

            let result = catch_unwind(AssertUnwindSafe(|| observer.on_next(boxed(Input))));

            assert_eq!(result.is_err(), panic_on_clone);
            assert!(dropped.get());
            assert_eq!(target.get_value(property).number, if panic_on_clone { 0 } else { 23 });
            assert_eq!(validations.get() == 0, panic_on_clone);
            binding.dispose();
        }
    }
}

#[test]
fn boxed_input_drop_runs_after_the_delivery_releases_its_binding() {
    use crate::data::core::ValueTypes;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    struct Input {
        reentrant: bool,
        on_drop: Option<Rc<dyn Fn()>>,
    }
    impl PartialEq for Input {
        fn eq(&self, other: &Self) -> bool {
            self.reentrant == other.reentrant
        }
    }
    impl Drop for Input {
        fn drop(&mut self) {
            if let Some(on_drop) = self.on_drop.take() {
                on_drop();
            }
        }
    }

    for priority in [BindingPriority::LocalValue, BindingPriority::Style] {
        for panic_on_conversion in [false, true] {
            let target = Class1::new();
            let property = FerroProperty::register::<Class1, _>("InputDropBindingLifetime", 0i32);
            let retained = Rc::new(RefCell::new(None::<Rc<dyn IObserver<BoxedValue>>>));
            let saved = retained.clone();
            let source = Observable::create(move |observer| {
                *saved.borrow_mut() = Some(observer);
                Disposable::empty()
            });
            let binding = target.bind_untyped(property, source, priority);
            let weak_binding = Rc::downgrade(&binding);
            let lease = Rc::new(RefCell::new(Some(binding)));
            let conversion_lease = lease.clone();
            let reentrant_conversions = Rc::new(Cell::new(0));
            let conversions = reentrant_conversions.clone();
            ValueTypes::register_cast::<Input, i32>(move |input| {
                if input.reentrant {
                    conversions.set(conversions.get() + 1);
                } else {
                    let binding = conversion_lease.borrow_mut().take().unwrap();
                    binding.dispose();
                    drop(binding);
                    assert!(!panic_on_conversion, "input conversion failed");
                }
                23
            });

            let observer = retained.borrow().as_ref().unwrap().clone();
            let reentrant_observer = observer.clone();
            let input_dropped = Rc::new(Cell::new(false));
            let dropped = input_dropped.clone();
            let alive_at_input_drop = Rc::new(Cell::new(false));
            let alive = alive_at_input_drop.clone();
            let input = Input {
                reentrant: false,
                on_drop: Some(Rc::new(move || {
                    dropped.set(true);
                    alive.set(weak_binding.upgrade().is_some());
                    // A source may retain its observer after disposal. The
                    // outer delivery must release its last binding reference
                    // before dropping this input, also while unwinding.
                    reentrant_observer.on_next(boxed(Input { reentrant: true, on_drop: None }));
                })),
            };
            let result = catch_unwind(AssertUnwindSafe(|| observer.on_next(boxed(input))));
            assert_eq!(result.is_err(), panic_on_conversion);
            assert!(lease.borrow().is_none());
            assert!(input_dropped.get());
            assert!(!alive_at_input_drop.get());
            assert_eq!(reentrant_conversions.get(), 0);
        }
    }
}
