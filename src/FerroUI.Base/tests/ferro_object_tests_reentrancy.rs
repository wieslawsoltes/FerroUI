//! Tests specific to this port: re-entrancy of the property system.
//!
//! Change notifications run arbitrary code that may set and clear properties,
//! add and remove bindings and handlers, and re-parent objects. None of that
//! may trip over interior borrows held by the property store, and the
//! observable behaviour must match upstream, where re-entrancy is unrestricted.

use super::*;
use crate::data::{BindingPriority, BindingValue, BindingValueType};
use crate::reactive::{IDisposable, ObservableExt};
use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    direct: RefCell<String>,
    validation: RefCell<Vec<(&'static str, BindingValueType, bool)>>,
}

ferro_class!(Class1: FerroObject);

impl FerroObjectImpl for Class1 {
    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&crate::data::BindingError>,
    ) {
        let name: &'static str = property.name();
        this.validation.borrow_mut().push((name, state, error.is_some()));
    }
}

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Bar", s("bardefault"))
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>("Baz", StyledPropertyOptions::new(s("bazdefault")).inherits(true))
    });

    // Coercion that reads the object while it runs.
    ferro_property!(pub fn coerced_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>(
            "Coerced",
            StyledPropertyOptions::new(0).coerce(|o, v| {
                let limit = o.get_value(Class1::limit_property());
                let _current = o.get_value(Class1::coerced_property());
                v.min(limit)
            }),
        )
    });

    ferro_property!(pub fn limit_property() -> StyledProperty<i32> {
        FerroProperty::register::<Class1, _>("Limit", 100)
    });

    ferro_property!(pub fn validated_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>(
            "Validated",
            StyledPropertyOptions::new(1).validate(|v| *v < 100).enable_data_validation(true),
        )
    });

    ferro_property!(pub fn droppy_property() -> StyledProperty<Droppy> {
        FerroProperty::register::<Class1, _>("Droppy", Droppy::new(0))
    });

    ferro_property!(pub fn direct_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct_with::<Class1, _>(
            "Direct",
            |o| o.direct.borrow().clone(),
            Some(|o, v| {
                o.set_and_raise(Class1::direct_property(), &o.direct, v);
            }),
            DirectPropertyMetadata::new(Some(s("unset"))).with_enable_data_validation(true),
        )
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            direct: RefCell::new(s("initial")),
            validation: RefCell::new(Vec::new()),
        })
    }

    pub fn with_parent(parent: &Ref<Class1>) -> Ref<Self> {
        let result = Self::new();
        result.set_inheritance_parent(parent);
        result
    }

    fn foo(&self) -> String {
        self.get_value(Self::foo_property())
    }

    fn baz(&self) -> String {
        self.get_value(Self::baz_property())
    }
}

thread_local! {
    static DROP_READER: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// A value that runs code when it is dropped.
#[derive(Debug)]
pub struct Droppy {
    id: i32,
}

impl Droppy {
    fn new(id: i32) -> Self {
        Self { id }
    }
}

impl Clone for Droppy {
    fn clone(&self) -> Self {
        Self { id: self.id }
    }
}

impl PartialEq for Droppy {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Drop for Droppy {
    fn drop(&mut self) {
        // Take the reader out while it runs: reading the property clones and
        // drops further values.
        let reader = DROP_READER.with(|r| r.borrow_mut().take());
        if let Some(reader) = reader {
            reader();
            DROP_READER.with(|r| *r.borrow_mut() = Some(reader));
        }
    }
}

type Changes = Recorder<(Option<String>, String)>;

fn record(target: &Ref<Class1>, property: &'static FerroProperty) -> Changes {
    let result: Changes = Recorder::new();
    let r = result.clone();
    target.property_changed(move |e| {
        if e.property() == property {
            r.push((old_string(e), new_string(e)));
        }
    });
    result
}

fn change(old: &str, new: &str) -> (Option<String>, String) {
    (Some(s(old)), s(new))
}

#[test]
fn handler_can_set_the_changing_property_again() {
    let target = Class1::new();
    let weak = target.downgrade();
    target.property_changed(move |e| {
        if new_string(e) == "a" {
            weak.upgrade().unwrap().set_value(Class1::foo_property(), s("b"));
        }
    });
    let changes = record(&target, Class1::foo_property());

    target.set_value(Class1::foo_property(), s("a"));

    assert_eq!("b", target.foo());
    // The nested change is delivered to later handlers before the change
    // that caused it, as with nested event invocations upstream.
    assert_eq!(vec![change("a", "b"), change("foodefault", "a")], changes.get());
}

#[test]
fn handler_can_clear_the_changing_property() {
    let target = Class1::new();
    let weak = target.downgrade();
    target.property_changed(move |e| {
        if new_string(e) == "a" {
            weak.upgrade().unwrap().clear_value(Class1::foo_property());
        }
    });

    target.set_value(Class1::foo_property(), s("a"));

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
}

#[test]
fn handler_can_set_other_properties_and_priorities() {
    let target = Class1::new();
    let weak = target.downgrade();
    target.property_changed(move |e| {
        let target = weak.upgrade().unwrap();
        if e.property() == Class1::foo_property().as_property() && new_string(e) == "a" {
            target.set_value(Class1::bar_property(), s("bar"));
            target.set_value_with_priority(Class1::foo_property(), s("anim"), BindingPriority::Animation);
            target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
        }
    });

    target.set_value(Class1::foo_property(), s("a"));

    assert_eq!("anim", target.foo());
    assert_eq!(Some(s("a")), target.get_base_value(Class1::foo_property()));
    assert_eq!("bar", target.get_value(Class1::bar_property()));

    // As upstream: clearing the local value has no effect while an
    // animation value is in effect.
    target.clear_value(Class1::foo_property());
    assert_eq!("anim", target.foo());
    assert_eq!(Some(s("a")), target.get_base_value(Class1::foo_property()));
}

#[test]
fn handler_can_add_and_remove_handlers() {
    let target = Class1::new();
    let late = Counter::new();
    let first: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let calls = Counter::new();

    let (weak, l, f, c) = (target.downgrade(), late.clone(), first.clone(), calls.clone());
    let subscription = target.property_changed(move |_| {
        c.increment();
        let l = l.clone();
        weak.upgrade().unwrap().property_changed(move |_| l.increment());
        if let Some(subscription) = f.borrow_mut().take() {
            subscription.dispose();
        }
    });
    *first.borrow_mut() = Some(subscription);

    target.set_value(Class1::foo_property(), s("a"));
    assert_eq!(1, calls.get());
    assert_eq!(0, late.get());

    target.set_value(Class1::foo_property(), s("b"));
    assert_eq!(1, calls.get());
    assert_eq!(1, late.get());
}

#[test]
fn handler_can_dispose_the_style_binding_that_produced_the_value() {
    let target = Class1::new();
    let source: Subject<String> = Subject::test(s("initial"));
    let binding: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

    let b = binding.clone();
    target.property_changed(move |e| {
        if new_string(e) == "kill" {
            if let Some(binding) = b.borrow_mut().take() {
                binding.dispose();
            }
        }
    });

    *binding.borrow_mut() = Some(target.bind(Class1::foo_property(), source.observable(), BindingPriority::Style));
    assert_eq!("initial", target.foo());

    source.on_next(s("kill"));

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
    assert_eq!(0, target.values().frames().len());
    assert_eq!(0, source.subscriber_count());

    // The store is still fully functional.
    target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);
    assert_eq!("style", target.foo());
}

#[test]
fn handler_can_dispose_the_local_binding_that_produced_the_value() {
    let target = Class1::new();
    let source: Subject<String> = Subject::test(s("initial"));
    let binding: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

    let b = binding.clone();
    target.property_changed(move |e| {
        if new_string(e) == "kill" {
            if let Some(binding) = b.borrow_mut().take() {
                binding.dispose();
            }
        }
    });

    *binding.borrow_mut() = Some(target.bind(Class1::foo_property(), source.observable(), BindingPriority::LocalValue));
    assert_eq!("initial", target.foo());

    source.on_next(s("kill"));

    assert_eq!("foodefault", target.foo());
    assert!(!target.is_set(Class1::foo_property()));
    assert_eq!(0, source.subscriber_count());
}

#[test]
fn handler_can_add_a_binding_to_the_changing_property() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation] {
        let target = Class1::new();
        let source = Subject::behavior(s("bound"));
        let done = Flag::new();

        let (weak, src, d) = (target.downgrade(), source.clone(), done.clone());
        target.property_changed(move |_| {
            if !d.get() {
                d.set(true);
                weak.upgrade().unwrap().bind(Class1::foo_property(), src.observable(), priority);
            }
        });

        target.set_value_with_priority(Class1::foo_property(), s("style"), BindingPriority::Style);

        assert_eq!("bound", target.foo(), "{priority:?}");

        source.on_next(s("next"));
        assert_eq!("next", target.foo(), "{priority:?}");
    }
}

#[test]
fn source_completing_during_notification_of_its_own_value() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation] {
        let target = Class1::new();
        let source: Subject<String> = Subject::new();

        let src = source.clone();
        target.property_changed(move |e| {
            if new_string(e) == "last" {
                src.on_completed();
            }
        });

        target.bind(Class1::foo_property(), source.observable(), priority);
        source.on_next(s("first"));
        assert_eq!("first", target.foo(), "{priority:?}");

        source.on_next(s("last"));

        assert_eq!("foodefault", target.foo(), "{priority:?}");
        assert!(!target.is_set(Class1::foo_property()), "{priority:?}");
        assert_eq!(0, target.values().frames().len(), "{priority:?}");
    }
}

#[test]
fn source_emitting_during_notification_of_its_own_value() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation] {
        let target = Class1::new();
        let source: Subject<String> = Subject::new();

        let src = source.clone();
        target.property_changed(move |e| {
            if new_string(e) == "first" {
                src.on_next(s("second"));
            }
        });

        target.bind(Class1::foo_property(), source.observable(), priority);
        source.on_next(s("first"));

        assert_eq!("second", target.foo(), "{priority:?}");
    }
}

#[test]
fn inherited_change_handler_on_parent_can_change_the_value_again() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    let weak = parent.downgrade();
    parent.property_changed(move |e| {
        if new_string(e) == "a" {
            weak.upgrade().unwrap().set_value(Class1::baz_property(), s("b"));
        }
    });
    let changes = record(&grandchild, Class1::baz_property());

    parent.set_value(Class1::baz_property(), s("a"));

    assert_eq!("b", parent.baz());
    assert_eq!("b", child.baz());
    assert_eq!("b", grandchild.baz());
    // Upstream propagates the value the parent has at the time of
    // propagation: the stale intermediate value is never announced as new.
    assert_eq!(vec![change("a", "b"), change("bazdefault", "b")], changes.get());
}

#[test]
fn inherited_change_handler_on_child_can_detach_and_reattach() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    let (weak_child, weak_parent) = (child.downgrade(), parent.downgrade());
    let (detached, reattached) = (Flag::new(), Flag::new());
    child.property_changed(move |e| {
        let child = weak_child.upgrade().unwrap();
        if new_string(e) == "a" && !detached.get() {
            detached.set(true);
            child.set_inheritance_parent(None);
        } else if new_string(e) == "bazdefault" && !reattached.get() {
            reattached.set(true);
            child.set_inheritance_parent(&weak_parent.upgrade().unwrap());
        }
    });

    parent.set_value(Class1::baz_property(), s("a"));

    assert_eq!("a", parent.baz());
    assert_eq!("a", child.baz());
    assert_eq!("a", grandchild.baz());
    assert!(child.inheritance_parent().is_some());

    parent.set_value(Class1::baz_property(), s("c"));
    assert_eq!("c", grandchild.baz());
}

#[test]
fn inherited_change_handler_on_child_can_set_and_clear_local_value() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    let weak = child.downgrade();
    let done = Flag::new();
    child.property_changed(move |e| {
        let child = weak.upgrade().unwrap();
        match new_string(e).as_str() {
            "a" if !done.get() => {
                done.set(true);
                child.set_value(Class1::baz_property(), s("local"));
            }
            "b" => child.clear_value(Class1::baz_property()),
            _ => {}
        }
    });

    parent.set_value(Class1::baz_property(), s("a"));
    assert_eq!("local", child.baz());
    assert_eq!("local", grandchild.baz());

    child.clear_value(Class1::baz_property());
    assert_eq!("a", child.baz());
    assert_eq!("a", grandchild.baz());

    parent.set_value(Class1::baz_property(), s("b"));
    assert_eq!("b", child.baz());
    assert_eq!("b", grandchild.baz());
    assert!(!child.is_set(Class1::baz_property()));
}

#[test]
fn handler_can_set_inheritance_parent_of_other_objects() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let orphan = Class1::new();

    let (weak_orphan, weak_child) = (orphan.downgrade(), child.downgrade());
    parent.property_changed(move |_| {
        // Adds an inheritance child to the object whose children are being
        // notified.
        weak_orphan.upgrade().unwrap().set_inheritance_parent(&weak_child.upgrade().unwrap());
    });

    parent.set_value(Class1::baz_property(), s("a"));

    assert_eq!("a", child.baz());
    assert_eq!("a", orphan.baz());
}

#[test]
fn direct_property_handler_can_set_the_property_again() {
    let target = Class1::new();
    let weak = target.downgrade();
    target.property_changed(move |e| {
        if new_string(e) == "a" {
            weak.upgrade().unwrap().set_direct_value(Class1::direct_property(), s("b"));
        }
    });

    target.set_direct_value(Class1::direct_property(), s("a"));

    assert_eq!("b", target.get_direct_value(Class1::direct_property()));
}

#[test]
fn direct_binding_can_be_disposed_from_handler() {
    let target = Class1::new();
    let source: Subject<String> = Subject::test(s("first"));
    let binding: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

    let b = binding.clone();
    target.property_changed(move |e| {
        if new_string(e) == "kill" {
            if let Some(binding) = b.borrow_mut().take() {
                binding.dispose();
            }
        }
    });

    *binding.borrow_mut() = Some(target.bind_direct(Class1::direct_property(), source.observable()));
    source.on_next(s("kill"));

    // As upstream: disposing a direct binding leaves the last value in place.
    assert_eq!("kill", target.get_direct_value(Class1::direct_property()));
    assert_eq!(0, source.subscriber_count());
}

#[test]
fn coercion_callback_can_read_the_object() {
    let target = Class1::new();

    target.set_value(Class1::coerced_property(), 150);
    assert_eq!(100, target.get_value(Class1::coerced_property()));

    target.set_value(Class1::limit_property(), 50);
    target.coerce_value(Class1::coerced_property());
    assert_eq!(50, target.get_value(Class1::coerced_property()));

    target.set_value_with_priority(Class1::coerced_property(), 70, BindingPriority::Animation);
    assert_eq!(50, target.get_value(Class1::coerced_property()));

    target.set_value(Class1::limit_property(), 200);
    target.coerce_value(Class1::coerced_property());
    assert_eq!(70, target.get_value(Class1::coerced_property()));
    // As upstream: when both the value and the base value change in a
    // coercion pass with an animation in effect, the base value is assigned
    // the coerced animated value (70) rather than the coerced base value
    // (150). This looks like an upstream defect; it is replicated, and this
    // assertion pins it so that a deliberate fix has to touch it.
    assert_eq!(Some(70), target.get_base_value(Class1::coerced_property()));
}

#[test]
fn observable_subscriber_can_unsubscribe_and_set_during_notification() {
    let target = Class1::new();
    let values: Recorder<String> = Recorder::new();
    let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

    let (v, sub, weak) = (values.clone(), subscription.clone(), target.downgrade());
    let observable = target.get_observable(Class1::foo_property());
    let result = observable.subscribe_fn(move |x| {
        v.push(x.clone());
        if x == "a" {
            weak.upgrade().unwrap().set_value(Class1::foo_property(), s("b"));
        }
        if x == "stop" {
            if let Some(subscription) = sub.borrow_mut().take() {
                subscription.dispose();
            }
        }
    });
    *subscription.borrow_mut() = Some(result);
    drop(observable);

    target.set_value(Class1::foo_property(), s("a"));
    assert_eq!("b", target.foo());

    target.set_value(Class1::foo_property(), s("stop"));
    target.set_value(Class1::foo_property(), s("after"));

    assert_eq!(vec![s("foodefault"), s("a"), s("b"), s("stop")], values.get());
}

#[test]
fn value_whose_drop_reads_the_property_does_not_trip_the_store() {
    let target = Class1::new();
    let weak = target.downgrade();
    DROP_READER.with(|r| {
        *r.borrow_mut() = Some(Box::new(move || {
            if let Some(target) = weak.upgrade() {
                let _ = target.get_value(Class1::droppy_property());
                let _ = target.get_base_value(Class1::droppy_property());
            }
        }));
    });

    target.set_value(Class1::droppy_property(), Droppy::new(1));
    target.set_value(Class1::droppy_property(), Droppy::new(2));
    target.set_value_with_priority(Class1::droppy_property(), Droppy::new(3), BindingPriority::Animation);
    target.set_value(Class1::droppy_property(), Droppy::new(4));
    target.set_current_value(Class1::droppy_property(), Droppy::new(5));

    let source: Subject<Droppy> = Subject::new();
    let binding = target.bind(Class1::droppy_property(), source.observable(), BindingPriority::Style);
    source.on_next(Droppy::new(6));
    source.on_next(Droppy::new(7));
    binding.dispose();

    // The animation value is still in effect.
    target.clear_value(Class1::droppy_property());
    assert_eq!(3, target.get_value(Class1::droppy_property()).id);

    DROP_READER.with(|r| *r.borrow_mut() = None);
}

// Upstream: a style binding value that fails validation becomes a binding
// error, which falls back to the default value and is reported as such to
// data validation.
#[test]
fn invalid_style_binding_value_is_reported_as_binding_error_with_fallback() {
    let target = Class1::new();
    let source: Subject<i32> = Subject::new();

    target.bind(Class1::validated_property(), source.observable(), BindingPriority::Style);
    source.on_next(5);
    source.on_next(150);

    assert_eq!(1, target.get_value(Class1::validated_property()));
    assert_eq!(
        vec![("Validated", BindingValueType::VALUE, false), ("Validated", BindingValueType::BINDING_ERROR_WITH_FALLBACK, true)],
        *target.validation.borrow()
    );

    let source: Subject<BindingValue<i32>> = Subject::new();
    let target = Class1::new();
    target.bind_value(Class1::validated_property(), source.observable(), BindingPriority::Style);
    source.on_next(BindingValue::new(150));

    assert_eq!(1, target.get_value(Class1::validated_property()));
    assert_eq!(
        vec![("Validated", BindingValueType::BINDING_ERROR_WITH_FALLBACK, true)],
        *target.validation.borrow()
    );
}

// Upstream: completing or disposing a direct binding clears the data
// validation state of a property with data validation enabled.
#[test]
fn completing_direct_binding_clears_data_validation() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::new();

    let binding = target.bind_direct_value(Class1::direct_property(), source.observable());
    source.on_next(BindingValue::data_validation_error(error("bad")));

    assert_eq!(vec![("Direct", BindingValueType::DATA_VALIDATION_ERROR, true)], *target.validation.borrow());

    binding.dispose();

    assert_eq!(
        vec![("Direct", BindingValueType::DATA_VALIDATION_ERROR, true), ("Direct", BindingValueType::UNSET_VALUE, false)],
        *target.validation.borrow()
    );
}

#[test]
fn handler_can_dispose_other_values_in_the_same_frame() {
    let target = Class1::new();
    let style = BindingPriority::Style;

    // Both values live in the same immediate value frame.
    let bar = target.set_value_with_priority(Class1::bar_property(), s("bar"), style).unwrap();
    let foo = target.set_value_with_priority(Class1::foo_property(), s("foo"), style).unwrap();
    assert_eq!(1, target.values().frames().len());

    let bar = RefCell::new(Some(bar));
    target.property_changed(move |e| {
        if e.property() == Class1::foo_property().as_property() {
            if let Some(bar) = bar.borrow_mut().take() {
                bar.dispose();
            }
        }
    });

    foo.dispose();

    assert_eq!("foodefault", target.foo());
    assert_eq!("bardefault", target.get_value(Class1::bar_property()));
    assert_eq!(0, target.values().frames().len());
    assert!(!target.is_set(Class1::foo_property()));
    assert!(!target.is_set(Class1::bar_property()));
}

#[test]
fn handler_can_set_the_property_while_it_is_being_cleared() {
    let target = Class1::new();
    target.set_value(Class1::foo_property(), s("a"));

    let weak = target.downgrade();
    target.property_changed(move |e| {
        if new_string(e) == "foodefault" {
            weak.upgrade().unwrap().set_value(Class1::foo_property(), s("again"));
        }
    });

    target.clear_value(Class1::foo_property());

    assert_eq!("again", target.foo());
    assert!(target.is_set(Class1::foo_property()));

    target.set_value(Class1::foo_property(), s("b"));
    assert_eq!("b", target.foo());
}

#[test]
fn handler_can_rebind_when_a_binding_completes() {
    for priority in [BindingPriority::LocalValue, BindingPriority::Style, BindingPriority::Animation] {
        let target = Class1::new();
        let first: Subject<String> = Subject::new();
        let second = Subject::behavior(s("second"));

        let (weak, src) = (target.downgrade(), second.clone());
        let done = Flag::new();
        target.property_changed(move |e| {
            if new_string(e) == "foodefault" && !done.get() {
                done.set(true);
                weak.upgrade().unwrap().bind(Class1::foo_property(), src.observable(), priority);
            }
        });

        target.bind(Class1::foo_property(), first.observable(), priority);
        first.on_next(s("first"));
        first.on_completed();

        assert_eq!("second", target.foo(), "{priority:?}");

        second.on_next(s("third"));
        assert_eq!("third", target.foo(), "{priority:?}");
    }
}

#[test]
fn dropped_inheritance_children_are_forgotten() {
    let parent = Class1::new();

    for _ in 0..10 {
        let child = Class1::with_parent(&parent);
        drop(child);
    }
    let child = Class1::with_parent(&parent);

    parent.set_value(Class1::baz_property(), s("a"));

    assert_eq!("a", child.baz());
    assert_eq!(1, parent.inheritance_children().len());
}

#[test]
fn invalid_priorities_are_rejected() {
    let target = Class1::new();

    for priority in [BindingPriority::Inherited, BindingPriority::Unset] {
        assert_panics(|| {
            target.set_value_with_priority(Class1::foo_property(), s("a"), priority);
        });
        assert_panics(|| {
            target.bind(Class1::foo_property(), Subject::<String>::new().observable(), priority);
        });
    }
}
