//! Port of the upstream `OnPropertyChanged` object tests.

use super::*;
use crate::data::{BindingPriority, BindingValue};
use crate::*;
use std::cell::RefCell;

/// A copy of a change notification of a string property.
#[derive(Clone, Debug)]
pub struct Change {
    old_value: Option<String>,
    new_value: String,
    priority: BindingPriority,
    is_effective_value_change: bool,
}

impl Change {
    fn clone_of(change: &FerroPropertyChangedEventArgs<'_>) -> Self {
        Self {
            old_value: change.get_old_value::<String>(),
            new_value: change.get_new_value::<String>(),
            priority: change.priority(),
            is_effective_value_change: change.is_effective_value_change(),
        }
    }
}

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    changes: RefCell<Vec<Change>>,
    core_changes: RefCell<Vec<Change>>,
}

ferro_class!(Class1: FerroObject);

impl FerroObjectImpl for Class1 {
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.core_changes.borrow_mut().push(Change::clone_of(change));
        Self::parent_on_property_changed_core(this, change);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.changes.borrow_mut().push(Change::clone_of(change));
        Self::parent_on_property_changed(this, change);
    }
}

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            changes: RefCell::new(Vec::new()),
            core_changes: RefCell::new(Vec::new()),
        })
    }
}

#[test]
fn on_property_changed_core_is_called_on_property_change() {
    let target = Class1::new();

    target.set_value(Class1::foo_property(), s("newvalue"));

    assert_eq!(1, target.core_changes.borrow().len());

    let change = target.core_changes.borrow()[0].clone();

    assert_eq!("newvalue", change.new_value);
    assert_eq!(Some(s("foodefault")), change.old_value);
    assert_eq!(BindingPriority::LocalValue, change.priority);
    assert!(change.is_effective_value_change);
}

#[test]
fn on_property_changed_core_is_called_on_non_effective_property_value_change() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("newvalue"), BindingPriority::Animation);
    target.set_value_with_priority(Class1::foo_property(), s("styled"), BindingPriority::Style);

    assert_eq!(2, target.core_changes.borrow().len());

    let change = target.core_changes.borrow()[1].clone();

    assert_eq!("styled", change.new_value);
    assert!(change.old_value.is_none());
    assert_eq!(BindingPriority::Style, change.priority);
    assert!(!change.is_effective_value_change);
}

#[test]
fn on_property_changed_core_is_called_on_non_effective_property_binding_value_change() {
    let target = Class1::new();
    let source: Subject<BindingValue<String>> = Subject::behavior(bv("styled1"));

    target.bind_value(Class1::foo_property(), source.observable(), BindingPriority::Style);
    target.set_value_with_priority(Class1::foo_property(), s("newvalue"), BindingPriority::Animation);
    source.on_next(bv("styled2"));

    assert_eq!(3, target.core_changes.borrow().len());

    let change = target.core_changes.borrow()[2].clone();

    assert_eq!("styled2", change.new_value);
    assert!(change.old_value.is_none());
    assert_eq!(BindingPriority::Style, change.priority);
    assert!(!change.is_effective_value_change);
}

#[test]
fn on_property_changed_is_called_only_for_effective_value_changes() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("newvalue"), BindingPriority::Animation);
    target.set_value_with_priority(Class1::foo_property(), s("styled"), BindingPriority::Style);

    assert_eq!(1, target.changes.borrow().len());
    assert_eq!(2, target.core_changes.borrow().len());
}
