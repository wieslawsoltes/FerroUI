//! Port of the upstream `Direct` object tests.

use super::*;
use crate::data::{BindingMode, BindingPriority, BindingValue};
use crate::reactive::ObservableExt;
use crate::*;
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    foo: RefCell<String>,
    bar: String,
    baz: Cell<i32>,
    double_value: Cell<f64>,
    frank: RefCell<Option<String>>,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct::<Class1, _>("Foo", |o| o.foo(), Some(|o, v| o.set_foo(v)), s("unset"))
    });

    ferro_property!(pub fn bar_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct::<Class1, _>("Bar", |o| o.bar(), None, String::new())
    });

    ferro_property!(pub fn baz_property() -> DirectProperty<Class1, i32> {
        FerroProperty::register_direct::<Class1, _>("Baz", |o| o.baz(), Some(|o, v| o.set_baz(v)), -1)
    });

    ferro_property!(pub fn double_value_property() -> DirectProperty<Class1, f64> {
        FerroProperty::register_direct::<Class1, _>(
            "DoubleValue",
            |o| o.double_value(),
            Some(|o, v| o.set_double_value(v)),
            0.0,
        )
    });

    // Upstream this is an `object` property.
    ferro_property!(pub fn frank_property() -> DirectProperty<Class1, Option<String>> {
        FerroProperty::register_direct::<Class1, _>(
            "Frank",
            |o| o.frank(),
            Some(|o, v| o.set_frank(v)),
            Some(s("Kups")),
        )
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            foo: RefCell::new(s("initial")),
            bar: s("bar"),
            baz: Cell::new(5),
            double_value: Cell::new(0.0),
            frank: RefCell::new(None),
        })
    }

    pub fn foo(&self) -> String {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: String) {
        self.set_and_raise(Self::foo_property(), &self.foo, value);
    }

    pub fn bar(&self) -> String {
        self.bar.clone()
    }

    pub fn baz(&self) -> i32 {
        self.baz.get()
    }

    pub fn set_baz(&self, value: i32) {
        self.set_and_raise_cell(Self::baz_property(), &self.baz, value);
    }

    pub fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    pub fn set_double_value(&self, value: f64) {
        self.set_and_raise_cell(Self::double_value_property(), &self.double_value, value);
    }

    pub fn frank(&self) -> Option<String> {
        self.frank.borrow().clone()
    }

    pub fn set_frank(&self, value: Option<String>) {
        self.set_and_raise(Self::frank_property(), &self.frank, value);
    }
}

#[repr(C)]
pub struct Class2 {
    base: FerroObject,
    foo: RefCell<String>,
}

ferro_class!(Class2: FerroObject);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class2 {
    ferro_property!(pub fn foo_property() -> DirectProperty<Class2, String> {
        Class1::foo_property().add_owner::<Class2>(|o| o.foo(), Some(|o, v| o.set_foo(v)), None)
    });

    pub fn new() -> Ref<Self> {
        // The equivalent of the static field initialisers running before the
        // first instance is created.
        let _ = Self::foo_property();
        instantiate(Self { base: FerroObject::construct(), foo: RefCell::new(s("initial2")) })
    }

    pub fn foo(&self) -> String {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: String) {
        self.set_and_raise(Self::foo_property(), &self.foo, value);
    }
}

fn untyped(source: &Subject<String>) -> std::rc::Rc<dyn crate::reactive::IObservable<BoxedValue>> {
    source.observable().select(|v| boxed(v))
}

#[test]
fn get_value_gets_default_value() {
    let target = Class1::new();

    assert_eq!("initial", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn get_value_gets_value_non_generic() {
    let target = Class1::new();

    let value = target.get_value_untyped(Class1::foo_property());
    assert_eq!(Some(&s("initial")), value.downcast_ref::<String>());
}

#[test]
fn get_value_on_unregistered_property_throws_exception() {
    let target = Class2::new();

    assert_panics(|| {
        target.get_direct_value(Class1::bar_property());
    });
}

#[test]
fn set_value_sets_value() {
    let target = Class1::new();

    target.set_direct_value(Class1::foo_property(), s("newvalue"));

    assert_eq!("newvalue", target.foo());
}

#[test]
fn set_value_sets_value_non_generic() {
    let target = Class1::new();

    target.set_value_untyped(Class1::foo_property(), &s("newvalue"), BindingPriority::LocalValue);

    assert_eq!("newvalue", target.foo());
}

#[test]
fn set_value_non_generic_coerces_unset_value_to_default_value() {
    let target = Class1::new();

    target.set_value_untyped(Class1::baz_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert_eq!(-1, target.baz());
}

#[test]
fn set_value_raises_property_changed() {
    let target = Class1::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.property_changed(move |e| {
        r.set(
            e.property() == Class1::foo_property().as_property()
                && old_string(e) == Some(s("initial"))
                && new_string(e) == "newvalue"
                && e.priority() == BindingPriority::LocalValue,
        );
    });

    target.set_direct_value(Class1::foo_property(), s("newvalue"));

    assert!(raised.get());
}

#[test]
fn set_value_raises_changed() {
    let target = Class1::new();
    let raised = Flag::new();

    let r = raised.clone();
    Class1::foo_property().changed().subscribe(move |e| {
        r.set(
            e.property() == Class1::foo_property().as_property()
                && old_string(e) == Some(s("initial"))
                && new_string(e) == "newvalue"
                && e.priority() == BindingPriority::LocalValue,
        );
    });

    target.set_direct_value(Class1::foo_property(), s("newvalue"));

    assert!(raised.get());
}

#[test]
fn setting_object_property_to_unset_value_reverts_to_default_value() {
    let target = Class1::new();

    target.set_value_untyped(Class1::frank_property(), &Some(s("newvalue")), BindingPriority::LocalValue);
    target.set_value_untyped(Class1::frank_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert_eq!(Some(s("Kups")), target.get_direct_value(Class1::frank_property()));
}

#[test]
fn setting_object_property_to_do_nothing_does_nothing() {
    let target = Class1::new();

    target.set_value_untyped(Class1::frank_property(), &Some(s("newvalue")), BindingPriority::LocalValue);
    target.set_value_untyped(Class1::frank_property(), &DoNothingType, BindingPriority::LocalValue);

    assert_eq!(Some(s("newvalue")), target.get_direct_value(Class1::frank_property()));
}

#[test]
fn bind_raises_property_changed() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();
    let raised = Flag::new();

    let r = raised.clone();
    target.property_changed(move |e| {
        r.set(
            e.property() == Class1::foo_property().as_property()
                && old_string(e) == Some(s("initial"))
                && new_string(e) == "newvalue"
                && e.priority() == BindingPriority::LocalValue,
        );
    });

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("newvalue"));

    assert!(raised.get());
}

#[test]
fn property_changed_not_raised_when_value_unchanged() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();
    let raised = Counter::new();

    let r = raised.clone();
    target.property_changed(move |_| r.increment());
    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("newvalue"));
    source.on_next(bv("newvalue"));

    assert_eq!(1, raised.get());
}

#[test]
fn set_value_on_unregistered_property_throws_exception() {
    let target = Class2::new();

    assert_panics(|| target.set_direct_value(Class1::bar_property(), s("value")));
}

#[test]
fn clear_value_restores_default_value() {
    let target = Class1::new();

    assert_eq!("initial", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn clear_value_raises_property_changed() {
    let target = Class1::new();
    let raised = Counter::new();

    target.set_direct_value(Class1::foo_property(), s("newvalue"));
    let (r, weak) = (raised.clone(), target.downgrade());
    target.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(BindingPriority::LocalValue, e.priority());
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(s("newvalue")), old_string(e));
        assert_eq!("unset", new_string(e));
        r.increment();
    });

    target.clear_direct_value(Class1::foo_property());

    assert_eq!(1, raised.get());
}

#[test]
fn get_observable_returns_values() {
    let target = Class1::new();
    let values: Recorder<String> = Recorder::new();

    let v = values.clone();
    target.get_observable(Class1::foo_property()).subscribe_fn(move |x| v.push(x));
    target.set_foo(s("newvalue"));

    assert_eq!(vec![s("initial"), s("newvalue")], values.get());
}

#[test]
fn bind_binds_property_value() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    let sub = target.bind_direct(Class1::foo_property(), source.observable());

    assert_eq!("initial", target.foo());
    source.on_next(s("first"));
    assert_eq!("first", target.foo());
    source.on_next(s("second"));
    assert_eq!("second", target.foo());

    sub.dispose();

    source.on_next(s("third"));
    assert_eq!("second", target.foo());
}

#[test]
fn bind_binds_property_value_non_generic() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    let sub = target.bind_property_untyped(Class1::foo_property(), untyped(&source), BindingPriority::LocalValue);

    assert_eq!("initial", target.foo());
    source.on_next(s("first"));
    assert_eq!("first", target.foo());
    source.on_next(s("second"));
    assert_eq!("second", target.foo());

    sub.dispose();

    source.on_next(s("third"));
    assert_eq!("second", target.foo());
}

#[test]
fn bind_non_generic_accepts_unset_value() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    let _sub = target.bind_property_untyped(Class1::baz_property(), source.observable(), BindingPriority::LocalValue);

    assert_eq!(5, target.baz());
    source.on_next(boxed(6));
    assert_eq!(6, target.baz());
    source.on_next(FerroProperty::unset_value());
    assert_eq!(-1, target.baz());
}

#[test]
fn bind_handles_wrong_type() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    let _sub = target.bind_direct_untyped(Class1::foo_property(), source.observable());

    source.on_next(boxed(45));

    assert_eq!("unset", target.foo());
}

#[test]
fn bind_handles_wrong_value_type() {
    let target = Class1::new();
    let source: Subject<BoxedValue> = Subject::new();

    let _sub = target.bind_direct_untyped(Class1::baz_property(), source.observable());

    source.on_next(boxed(s("foo")));

    assert_eq!(-1, target.baz());
}

#[test]
fn read_only_property_cannot_be_set() {
    let target = Class1::new();

    assert_panics(|| target.set_direct_value(Class1::bar_property(), s("newvalue")));
}

#[test]
fn read_only_property_cannot_be_set_non_generic() {
    let target = Class1::new();

    assert_panics(|| {
        target.set_value_untyped(Class1::bar_property(), &s("newvalue"), BindingPriority::LocalValue);
    });
}

#[test]
fn read_only_property_cannot_be_bound() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    assert_panics(|| {
        target.bind_direct(Class1::bar_property(), source.observable());
    });
}

#[test]
fn read_only_property_cannot_be_bound_non_generic() {
    let target = Class1::new();
    let source: Subject<String> = Subject::new();

    assert_panics(|| {
        target.bind_property_untyped(Class1::bar_property(), untyped(&source), BindingPriority::LocalValue);
    });
}

#[test]
fn get_value_gets_value_on_add_ownered_property() {
    let target = Class2::new();

    assert_eq!("initial2", target.get_direct_value(Class2::foo_property()));
}

#[test]
fn get_value_gets_value_on_add_ownered_property_using_original() {
    let target = Class2::new();

    assert_eq!("initial2", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn get_value_gets_value_on_add_ownered_property_using_original_non_generic() {
    let target = Class2::new();

    let value = target.get_value_untyped(Class1::foo_property());
    assert_eq!(Some(&s("initial2")), value.downcast_ref::<String>());
}

#[test]
fn set_value_sets_value_on_add_ownered_property_using_original() {
    let target = Class2::new();

    target.set_direct_value(Class1::foo_property(), s("newvalue"));

    assert_eq!("newvalue", target.foo());
}

#[test]
fn set_value_sets_value_on_add_ownered_property_using_original_non_generic() {
    let target = Class2::new();

    target.set_value_untyped(Class1::foo_property(), &s("newvalue"), BindingPriority::LocalValue);

    assert_eq!("newvalue", target.foo());
}

#[test]
fn unset_value_is_used_on_add_ownered_property() {
    let target = Class2::new();

    target.set_value_untyped(Class1::foo_property(), &UnsetValueType, BindingPriority::LocalValue);

    assert_eq!("unset", target.foo());
}

#[test]
fn bind_binds_add_ownered_property_value() {
    let target = Class2::new();
    let source: Subject<String> = Subject::new();

    let sub = target.bind_direct(Class1::foo_property(), source.observable());

    assert_eq!("initial2", target.foo());
    source.on_next(s("first"));
    assert_eq!("first", target.foo());
    source.on_next(s("second"));
    assert_eq!("second", target.foo());

    sub.dispose();

    source.on_next(s("third"));
    assert_eq!("second", target.foo());
}

#[test]
fn bind_binds_add_ownered_property_value_non_generic() {
    let target = Class2::new();
    let source: Subject<String> = Subject::new();

    let sub = target.bind_property_untyped(Class1::foo_property(), untyped(&source), BindingPriority::LocalValue);

    assert_eq!("initial2", target.foo());
    source.on_next(s("first"));
    assert_eq!("first", target.foo());
    source.on_next(s("second"));
    assert_eq!("second", target.foo());

    sub.dispose();

    source.on_next(s("third"));
    assert_eq!("second", target.foo());
}

#[test]
fn binding_error_reverts_to_default_value() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("initial"));
    source.on_next(BindingValue::binding_error(error("Foo")));

    assert_eq!("unset", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn binding_error_with_fallback_value_causes_target_update() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("initial"));
    source.on_next(BindingValue::binding_error_with_fallback(error("Foo"), Some(s("bar"))));

    assert_eq!("bar", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn data_validation_error_does_not_cause_target_update() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("initial"));
    source.on_next(BindingValue::data_validation_error(error("Foo")));

    assert_eq!("initial", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn data_validation_error_with_fallback_value_causes_target_update() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("initial"));
    source.on_next(BindingValue::data_validation_error_with_fallback(error("Foo"), Some(s("bar"))));

    assert_eq!("bar", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn binding_error_with_fallback_value_causes_target_update_2() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    target.bind_direct_value(Class1::foo_property(), source.observable());
    source.on_next(bv("initial"));
    source.on_next(BindingValue::binding_error_with_fallback(error("Foo"), Some(s("fallback"))));

    assert_eq!("fallback", target.get_direct_value(Class1::foo_property()));
}

#[test]
fn add_owner_should_inherit_default_binding_mode() {
    let foo = DirectProperty::<Class1, String>::create(
        "foo",
        |_| s("foo"),
        None,
        DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::TwoWay),
    );
    let bar = foo.add_owner::<Class2>(|_| s("bar"), None, None);

    assert_eq!(BindingMode::TwoWay, bar.get_metadata(Class1::TYPE).default_binding_mode());
    assert_eq!(BindingMode::TwoWay, bar.get_metadata(Class2::TYPE).default_binding_mode());
}

#[test]
fn add_owner_can_override_default_binding_mode() {
    let foo = DirectProperty::<Class1, String>::create(
        "foo",
        |_| s("foo"),
        None,
        DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::TwoWay),
    );
    let bar = foo.add_owner::<Class2>(
        |_| s("bar"),
        None,
        Some(DirectPropertyMetadata::new(None).with_default_binding_mode(BindingMode::OneWayToSource)),
    );

    assert_eq!(BindingMode::TwoWay, bar.get_metadata(Class1::TYPE).default_binding_mode());
    assert_eq!(BindingMode::OneWayToSource, bar.get_metadata(Class2::TYPE).default_binding_mode());
}

// Not from upstream: the untyped route of a direct property converts a value
// implicitly, as `DirectPropertyBase<TValue>.RouteSetValue` does through
// `TryConvert` (the styled route has upstream's tests in
// `ferro_object_tests_set_value.rs`).
#[test]
fn set_value_untyped_converts_a_value_implicitly() {
    let target = Class1::new();

    // An integer on a double property.
    target.set_value_untyped(Class1::double_value_property(), &4, BindingPriority::LocalValue);
    assert_eq!(4.0, target.get_direct_value(Class1::double_value_property()));

    // A value on a nullable property.
    target.set_value_untyped(Class1::frank_property(), &s("newvalue"), BindingPriority::LocalValue);
    assert_eq!(Some(s("newvalue")), target.get_direct_value(Class1::frank_property()));
}

// Not from upstream: a value that has no implicit conversion to the type of a
// direct property is refused, as the managed original throws.
#[test]
fn set_value_untyped_refuses_a_value_without_an_implicit_conversion() {
    let target = Class1::new();

    assert_panics(|| {
        target.set_value_untyped(Class1::baz_property(), &4.5, BindingPriority::LocalValue);
    });
}
