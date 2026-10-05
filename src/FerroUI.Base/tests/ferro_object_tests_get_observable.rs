//! Port of the upstream `GetObservable` object tests.

use super::*;
use crate::reactive::ObservableExt;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });
}

test_class!(Class2: Class1);

impl Class2 {
    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class2, _>("Bar", s("bardefault"))
    });
}

#[test]
fn get_observable_returns_initial_value() {
    let target = Class1::new();
    let raised = Counter::new();

    let r = raised.clone();
    target.get_observable(Class1::foo_property()).subscribe_fn(move |x| {
        if x == "foodefault" {
            r.increment();
        }
    });

    assert_eq!(1, raised.get());
}

#[test]
fn get_observable_returns_property_change() {
    let target = Class1::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.get_observable(Class1::foo_property()).subscribe_fn(move |x| r.set(x == "newvalue"));
    raised.set(false);
    target.set_value(Class1::foo_property(), s("newvalue"));

    assert!(raised.get());
}

#[test]
fn get_observable_returns_property_change_only_for_correct_property() {
    let target = Class2::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.get_observable(Class1::foo_property()).subscribe_fn(move |_| r.set(true));
    raised.set(false);
    target.set_value(Class2::bar_property(), s("newvalue"));

    assert!(!raised.get());
}

#[test]
fn get_observable_dispose_stops_property_changes() {
    let target = Class1::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.get_observable(Class1::foo_property()).subscribe_fn(move |_| r.set(true)).dispose();
    raised.set(false);
    target.set_value(Class1::foo_property(), s("newvalue"));

    assert!(!raised.get());
}

// The tests below are not upstream tests: they cover the remaining members
// of the object extensions.

#[test]
fn get_observable_with_converter_and_untyped_return_values() {
    let target = Class1::new();
    let lengths: Recorder<usize> = Recorder::new();
    let untyped: Recorder<String> = Recorder::new();

    let l = lengths.clone();
    target.get_observable_with(Class1::foo_property(), |v| v.len()).subscribe_fn(move |x| l.push(x));
    let u = untyped.clone();
    target
        .get_observable_untyped(Class1::foo_property())
        .subscribe_fn(move |x| u.push(x.downcast_ref::<String>().expect("a string").clone()));

    target.set_value(Class1::foo_property(), s("ab"));
    // The projected value is unchanged, so nothing is published for it.
    target.set_value(Class1::foo_property(), s("cd"));

    assert_eq!(vec![10, 2], lengths.get());
    assert_eq!(vec![s("foodefault"), s("ab"), s("cd")], untyped.get());
}

#[test]
fn get_binding_observable_returns_binding_values() {
    let target = Class1::new();
    let values: Recorder<crate::data::BindingValue<String>> = Recorder::new();

    let v = values.clone();
    let subscription = target.get_binding_observable(Class1::foo_property()).subscribe_fn(move |x| v.push(x));
    target.set_value(Class1::foo_property(), s("newvalue"));
    subscription.dispose();
    target.set_value(Class1::foo_property(), s("ignored"));

    assert_eq!(vec![bv("foodefault"), bv("newvalue")], values.get());
}

#[test]
fn get_property_changed_observable_fires_only_for_the_property_and_supports_class_handlers() {
    let target = Class2::new();
    let changes: Recorder<(Option<String>, String)> = Recorder::new();
    let class1 = Counter::new();
    let class2 = Counter::new();

    let observable = target.get_property_changed_observable(Class1::foo_property());
    let c = changes.clone();
    let subscription = observable.subscribe(move |e| c.push((old_string(e), new_string(e))));
    let c1 = class1.clone();
    observable.add_class_handler::<Class1>(move |_, _| c1.increment());
    let c2 = class2.clone();
    target
        .get_property_changed_observable(Class2::bar_property())
        .add_class_handler::<Class2>(move |_, _| c2.increment());

    target.set_value(Class1::foo_property(), s("a"));
    target.set_value(Class2::bar_property(), s("b"));
    subscription.dispose();
    target.set_value(Class1::foo_property(), s("c"));

    assert_eq!(vec![(Some(s("foodefault")), s("a"))], changes.get());
    assert_eq!(2, class1.get());
    assert_eq!(1, class2.get());
}

#[test]
fn observable_does_not_keep_the_object_alive() {
    let target = Class1::new();
    let weak = target.downgrade();
    let observable = target.get_observable(Class1::foo_property());
    let _subscription = observable.subscribe_fn(|_| {});

    drop(target);

    assert!(weak.upgrade().is_none());
}

#[test]
fn typed_accessors_route_by_property_kind() {
    use crate::data::BindingPriority;

    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    let binding = target.bind_typed(Class1::foo_property(), source.observable(), BindingPriority::Style);
    source.on_next(s("style"));
    target.set_value_with_priority(Class1::foo_property(), s("anim"), BindingPriority::Animation);

    assert_eq!("anim", target.get_typed_value(Class1::foo_property()));
    assert_eq!(Some(s("style")), target.get_typed_base_value(Class1::foo_property()));
    assert_eq!(
        Some(&s("style")),
        target.get_base_value_untyped(Class1::foo_property()).downcast_ref::<String>()
    );

    binding.dispose();

    let other = Class1::new();
    assert!(other.get_typed_base_value(Class1::foo_property()).is_none());
    assert!(other.get_base_value_untyped(Class1::foo_property()).is::<UnsetValueType>());
}
