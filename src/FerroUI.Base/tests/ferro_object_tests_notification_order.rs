//! Tests specific to this port: the sequence of the notifications of a
//! property change.
//!
//! Upstream raises a change in a fixed order (`RaisePropertyChanged`): the
//! `OnPropertyChangedCore` override, which calls the `OnPropertyChanged`
//! overrides for a change of the effective value; then the subscribers of the
//! property (`Changed`, class handlers among them) in the order they
//! subscribed; then the `PropertyChanged` handlers of the object in the order
//! they were added. A change made by one of them is raised completely before
//! the next of them is called, and the subscribers and handlers called for a
//! change are the ones there were when it began to call them.
//!
//! The tests record whole sequences, so that a change of the machinery that
//! carries the notifications (the dispatch of the overrides, the lists of
//! handlers) is held to them.

use super::*;
use crate::data::BindingPriority;
use crate::reactive::IDisposable;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    /// What was called on the thread, in order.
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: impl Into<String>) {
    LOG.with(|log| log.borrow_mut().push(entry.into()));
}

fn logged() -> Vec<String> {
    LOG.with(|log| log.borrow().clone())
}

fn clear_log() {
    LOG.with(|log| log.borrow_mut().clear());
}

/// The property and the values of a change of a string property.
fn describe(change: &FerroPropertyChangedEventArgs<'_>) -> String {
    format!(
        "{} {} to {}{}",
        change.property().name(),
        old_string(change).unwrap_or_else(|| s("-")),
        new_string(change),
        if change.is_effective_value_change() { "" } else { " (base)" }
    )
}

/// The name of the object a change was raised on.
fn sender_name(change: &FerroPropertyChangedEventArgs<'_>) -> &'static str {
    change.sender().downcast_ref::<NotifyOrderBase>().expect("the sender is a test object").name
}

#[repr(C)]
pub struct NotifyOrderBase {
    base: FerroObject,
    name: &'static str,
}

ferro_class!(NotifyOrderBase: FerroObject);

impl FerroObjectImpl for NotifyOrderBase {
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log(format!("{}.core> {}", this.name, describe(change)));
        Self::parent_on_property_changed_core(this, change);
        log(format!("{}.core<", this.name));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log(format!("{}.base", this.name));
        Self::parent_on_property_changed(this, change);
    }
}

impl NotifyOrderBase {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<NotifyOrderBase, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<String> {
        FerroProperty::register::<NotifyOrderBase, _>("Bar", s("bardefault"))
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register_with::<NotifyOrderBase, _>(
            "Baz",
            StyledPropertyOptions::new(s("bazdefault")).inherits(true),
        )
    });

    fn construct(name: &'static str) -> Self {
        Self { base: FerroObject::construct(), name }
    }
}

#[repr(C)]
pub struct NotifyOrderDerived {
    base: NotifyOrderBase,
}

ferro_class!(NotifyOrderDerived: NotifyOrderBase);

impl FerroObjectImpl for NotifyOrderDerived {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log(format!("{}.derived>", this.name));
        Self::parent_on_property_changed(this, change);
        log(format!("{}.derived<", this.name));
    }
}

impl NotifyOrderDerived {
    pub fn new(name: &'static str) -> Ref<Self> {
        instantiate(Self { base: NotifyOrderBase::construct(name) })
    }

    pub fn with_parent(name: &'static str, parent: &Ref<NotifyOrderDerived>) -> Ref<Self> {
        let result = Self::new(name);
        result.set_inheritance_parent(parent);
        result
    }
}

/// The entries of the overrides of `name` for one change of the effective
/// value: the core override around the overrides of the derived and of the
/// base class.
fn overrides(name: &str, change: &str) -> Vec<String> {
    vec![
        format!("{name}.core> {change}"),
        format!("{name}.derived>"),
        format!("{name}.base"),
        format!("{name}.derived<"),
        format!("{name}.core<"),
    ]
}

fn sequence(parts: Vec<Vec<String>>) -> Vec<String> {
    parts.into_iter().flatten().collect()
}

fn entries(entries: &[&str]) -> Vec<String> {
    entries.iter().map(|entry| entry.to_string()).collect()
}

#[test]
fn change_is_raised_to_the_overrides_then_the_property_subscribers_then_the_object_handlers() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();

    foo.changed().add_class_handler::<NotifyOrderDerived>(|_, e| log(format!("class1 {}", describe(e))));
    foo.changed().subscribe(|e| log(format!("property2 {}", describe(e))));
    foo.changed().add_class_handler::<NotifyOrderBase>(|_, e| log(format!("class3 {}", describe(e))));
    target.property_changed(|e| log(format!("handler1 {}", describe(e))));
    target.property_changed(|e| log(format!("handler2 {}", describe(e))));

    target.set_value(foo, s("a"));

    assert_eq!(
        sequence(vec![
            overrides("t", "Foo foodefault to a"),
            entries(&[
                "class1 Foo foodefault to a",
                "property2 Foo foodefault to a",
                "class3 Foo foodefault to a",
                "handler1 Foo foodefault to a",
                "handler2 Foo foodefault to a",
            ]),
        ]),
        logged()
    );
}

#[test]
fn class_handler_is_called_for_objects_of_its_class_only() {
    let base = instantiate(NotifyOrderBase::construct("b"));
    let derived = NotifyOrderDerived::new("d");
    let foo = NotifyOrderBase::foo_property();

    foo.changed().add_class_handler::<NotifyOrderDerived>(|_, e| log(format!("derived {}", sender_name(e))));
    foo.changed().add_class_handler::<NotifyOrderBase>(|_, e| log(format!("base {}", sender_name(e))));

    base.set_value(foo, s("a"));
    derived.set_value(foo, s("a"));

    assert_eq!(
        entries(&[
            "b.core> Foo foodefault to a",
            "b.base",
            "b.core<",
            "base b",
            "d.core> Foo foodefault to a",
            "d.derived>",
            "d.base",
            "d.derived<",
            "d.core<",
            "derived d",
            "base d",
        ]),
        logged()
    );
}

#[test]
fn change_of_a_base_value_is_raised_to_the_core_override_only() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();

    foo.changed().subscribe(|e| log(format!("property {}", describe(e))));
    target.property_changed(|e| log(format!("handler {}", describe(e))));

    target.set_value_with_priority(foo, s("animated"), BindingPriority::Animation);
    clear_log();

    // The animated value stays in effect: only the base value changes.
    target.set_value_with_priority(foo, s("styled"), BindingPriority::Style);

    assert_eq!(entries(&["t.core> Foo - to styled (base)", "t.core<"]), logged());
}

#[test]
fn change_made_by_a_property_subscriber_is_raised_completely_before_the_next_subscriber() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();
    let bar = NotifyOrderBase::bar_property();

    foo.changed().add_class_handler::<NotifyOrderDerived>(|o, e| {
        log(format!("class1 {}", describe(e)));
        o.set_value(NotifyOrderBase::bar_property(), s("x"));
    });
    foo.changed().subscribe(|e| log(format!("property2 {}", describe(e))));
    bar.changed().subscribe(|e| log(format!("bar {}", describe(e))));
    target.property_changed(|e| log(format!("handler {}", describe(e))));

    target.set_value(foo, s("a"));

    assert_eq!(
        sequence(vec![
            overrides("t", "Foo foodefault to a"),
            entries(&["class1 Foo foodefault to a"]),
            overrides("t", "Bar bardefault to x"),
            entries(&[
                "bar Bar bardefault to x",
                "handler Bar bardefault to x",
                "property2 Foo foodefault to a",
                "handler Foo foodefault to a",
            ]),
        ]),
        logged()
    );
    assert_eq!("x", target.get_value(bar));
}

#[test]
fn change_of_the_same_property_made_by_a_handler_reaches_later_handlers_first() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();

    foo.changed().subscribe(|e| log(format!("property {}", describe(e))));
    let weak = target.downgrade();
    target.property_changed(move |e| {
        log(format!("handler1 {}", describe(e)));
        if new_string(e) == "a" {
            weak.upgrade().unwrap().set_value(NotifyOrderBase::foo_property(), s("b"));
        }
    });
    target.property_changed(|e| log(format!("handler2 {}", describe(e))));

    target.set_value(foo, s("a"));

    // The outer change keeps the values it was raised with.
    assert_eq!(
        sequence(vec![
            overrides("t", "Foo foodefault to a"),
            entries(&["property Foo foodefault to a", "handler1 Foo foodefault to a"]),
            overrides("t", "Foo a to b"),
            entries(&[
                "property Foo a to b",
                "handler1 Foo a to b",
                "handler2 Foo a to b",
                "handler2 Foo foodefault to a",
            ]),
        ]),
        logged()
    );
    assert_eq!("b", target.get_value(foo));
}

#[test]
fn property_subscribers_called_for_a_change_are_the_ones_there_were_when_it_began() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();
    let second_calls = Counter::new();
    let late_calls = Counter::new();
    let second: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let done = Rc::new(Cell::new(false));

    let (second_slot, late) = (second.clone(), late_calls.clone());
    foo.changed().subscribe(move |_| {
        if !done.replace(true) {
            // Subscribes another and unsubscribes the next one.
            let late = late.clone();
            NotifyOrderBase::foo_property().changed().subscribe(move |_| late.increment());
            if let Some(subscription) = second_slot.borrow_mut().take() {
                subscription.dispose();
            }
        }
    });
    let calls = second_calls.clone();
    *second.borrow_mut() = Some(foo.changed().subscribe(move |_| calls.increment()));

    // The subscriber that was unsubscribed during the change is still called
    // for it; the one that subscribed during it is not.
    target.set_value(foo, s("a"));
    assert_eq!(1, second_calls.get());
    assert_eq!(0, late_calls.get());

    target.set_value(foo, s("b"));
    assert_eq!(1, second_calls.get());
    assert_eq!(1, late_calls.get());
}

#[test]
fn object_handlers_called_for_a_change_are_the_ones_there_were_when_it_began() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();
    let second_calls = Counter::new();
    let late_calls = Counter::new();
    let second: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
    let done = Rc::new(Cell::new(false));

    let (weak, second_slot, late) = (target.downgrade(), second.clone(), late_calls.clone());
    target.property_changed(move |_| {
        if !done.replace(true) {
            // Adds another handler and removes the next one.
            let late = late.clone();
            weak.upgrade().unwrap().property_changed(move |_| late.increment());
            if let Some(subscription) = second_slot.borrow_mut().take() {
                subscription.dispose();
            }
        }
    });
    let calls = second_calls.clone();
    *second.borrow_mut() = Some(target.property_changed(move |_| calls.increment()));
    assert_eq!(2, target.property_changed_subscriber_count());

    // The handler that was removed during the change is still called for it;
    // the one that was added during it is not.
    target.set_value(foo, s("a"));
    assert_eq!(1, second_calls.get());
    assert_eq!(0, late_calls.get());
    assert_eq!(2, target.property_changed_subscriber_count());

    target.set_value(foo, s("b"));
    assert_eq!(1, second_calls.get());
    assert_eq!(1, late_calls.get());
}

#[test]
fn handler_added_inside_a_nested_change_is_called_for_neither_change() {
    let target = NotifyOrderDerived::new("t");
    let foo = NotifyOrderBase::foo_property();
    let late_calls = Counter::new();

    let (weak, late) = (target.downgrade(), late_calls.clone());
    target.property_changed(move |e| {
        let target = weak.upgrade().unwrap();
        match new_string(e).as_str() {
            "a" => target.set_value(NotifyOrderBase::foo_property(), s("b")),
            "b" => {
                let late = late.clone();
                target.property_changed(move |_| late.increment());
            }
            _ => {}
        }
    });

    target.set_value(foo, s("a"));
    assert_eq!(0, late_calls.get());

    target.set_value(foo, s("c"));
    assert_eq!(1, late_calls.get());
}

#[test]
fn inherited_change_is_raised_completely_on_an_object_before_its_children() {
    let parent = NotifyOrderDerived::new("p");
    let child1 = NotifyOrderDerived::with_parent("c1", &parent);
    let grandchild = NotifyOrderDerived::with_parent("g", &child1);
    let child2 = NotifyOrderDerived::with_parent("c2", &parent);
    let baz = NotifyOrderBase::baz_property();

    baz.changed().subscribe(|e| log(format!("property {}", sender_name(e))));
    for object in [&parent, &child1, &grandchild, &child2] {
        object.property_changed(|e| log(format!("handler {} {}", sender_name(e), describe(e))));
    }
    clear_log();

    parent.set_value(baz, s("v"));

    let on = |name: &str| {
        sequence(vec![
            overrides(name, "Baz bazdefault to v"),
            vec![format!("property {name}"), format!("handler {name} Baz bazdefault to v")],
        ])
    };
    assert_eq!(sequence(vec![on("p"), on("c1"), on("g"), on("c2")]), logged());
    assert_eq!("v", grandchild.get_value(baz));
}

#[test]
fn inherited_change_stops_at_an_object_that_sets_the_property() {
    let parent = NotifyOrderDerived::new("p");
    let child = NotifyOrderDerived::with_parent("c", &parent);
    let grandchild = NotifyOrderDerived::with_parent("g", &child);
    let baz = NotifyOrderBase::baz_property();

    child.set_value(baz, s("own"));
    clear_log();

    parent.set_value(baz, s("v"));

    assert_eq!(overrides("p", "Baz bazdefault to v"), logged());
    assert_eq!("own", grandchild.get_value(baz));
}
