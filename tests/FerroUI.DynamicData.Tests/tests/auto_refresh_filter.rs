//! `AutoRefreshFilter` (`DynamicData.Tests/AutoRefreshFilter.cs`): `AutoRefresh` of lists followed
//! by a filter, static or dynamic, with the types `Item` and `ActivableItem` of the file.

use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::binding::{AbstractNotifyPropertyChanged, ObservableCollectionEx, ObservableCollectionExtended};
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::{prop, IObserver};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// An item with a name that raises `PropertyChanged` for `Name` on every assignment (`Item`). Items
/// are held in `Rc` and are equal only to themselves, as the reference type of upstream. Upstream's
/// `Id` (a new `Guid`), which no test reads, is not ported.
struct Item {
    notify: AbstractNotifyPropertyChanged,
    name: RefCell<String>,
}

impl Item {
    /// `new Item(name)`.
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), name: RefCell::new(name.to_owned()) })
    }

    /// `Name`.
    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// Sets `Name` and raises `Name`, whether or not it changed.
    fn set_name(&self, value: &str) {
        *self.name.borrow_mut() = value.to_owned();
        self.notify.on_property_changed("Name");
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item").field("name", &*self.name.borrow()).finish()
    }
}

/// An item that raises `PropertyChanged` for `Activated` on every assignment (`ActivableItem`).
struct ActivableItem {
    notify: AbstractNotifyPropertyChanged,
    activated: Cell<bool>,
}

impl ActivableItem {
    /// `new ActivableItem { Activated = activated }`.
    fn new(activated: bool) -> Rc<Self> {
        let item = Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), activated: Cell::new(false) });
        item.set_activated(activated);
        item
    }

    /// `Activated`.
    fn activated(&self) -> bool {
        self.activated.get()
    }

    /// Sets `Activated` and raises `Activated`, whether or not it changed.
    fn set_activated(&self, value: bool) {
        self.activated.set(value);
        self.notify.on_property_changed("Activated");
    }
}

impl INotifyPropertyChanged for ActivableItem {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for ActivableItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for ActivableItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivableItem").field("activated", &self.activated.get()).finish()
    }
}

type ActivablePredicate = Rc<dyn Fn(&Rc<ActivableItem>) -> bool>;

#[test]
fn bind_transform_and_filter_on_observable() {
    let count = 3;
    let list = SourceList::<String>::new();
    list.edit(|items| items.add_range((1..=count).map(|c| format!("item {c}")).collect()));

    let binded_list = ObservableCollectionExtended::<String>::new();

    let _subscription = list
        .connect()
        .filter_on_observable(|_: &String| Observable::return_(true))
        .transform(|s: &String| s.clone())
        .bind(&binded_list)
        .subscribe_next_error(|_| {}, |_| panic!("There should be no error"));
}

#[test]
fn test() {
    let a0 = Item::new("A0");
    let i1 = Item::new("I1");
    let i2 = Item::new("I2");
    let i3 = Item::new("I3");

    let obs_list = SourceList::<Rc<Item>>::new();
    obs_list.edit(|items| items.add_range(vec![a0.clone(), i1.clone(), i2.clone(), i3.clone()]));

    let obs_list_derived = obs_list.connect().auto_refresh(prop!(Item => name)).filter(|x: &Rc<Item>| x.name().contains('I')).as_observable_list();

    assert_eq!(obs_list_derived.count(), 3);
    assert_equivalent(&obs_list_derived.items(), &[i1.clone(), i2.clone(), i3.clone()]);

    i1.set_name("X2");
    assert_eq!(obs_list_derived.count(), 2);
    assert_equivalent(&obs_list_derived.items(), &[i2.clone(), i3.clone()]);

    a0.set_name("I0");
    assert_eq!(obs_list_derived.count(), 3);
    assert_equivalent(&obs_list_derived.items(), &[a0.clone(), i2.clone(), i3.clone()]);
}

#[test]
fn auto_refresh_with_observable_predicate1() {
    let item1 = ActivableItem::new(false);

    let items = SourceList::<Rc<ActivableItem>>::new();
    items.edit(|list| list.add(item1.clone()));

    let filter_subject = BehaviorSubject::new(Rc::new(|_: &Rc<ActivableItem>| false) as ActivablePredicate);

    let obs_list_derived = items
        .connect()
        .auto_refresh(prop!(ActivableItem => activated))
        .filter_dynamic(filter_subject.as_observable())
        .do_(|x: &ChangeSet<Rc<ActivableItem>>| println!("Changes{}", x.total_changes()))
        .as_observable_list();

    // Default filter predicate denies all items
    // The binding collection should stay empty, until the predicate changes
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(true);
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(false);
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(true);
    assert_eq!(obs_list_derived.count(), 0);

    // Changing predicate, all "Activated" items should added to the binding collection
    filter_subject.on_next(Rc::new(|i: &Rc<ActivableItem>| i.activated()) as ActivablePredicate);

    assert_eq!(obs_list_derived.count(), 1);
    assert_equivalent(&obs_list_derived.items(), &[item1.clone()]);

    // Changing property value
    item1.set_activated(false);
    assert_eq!(obs_list_derived.count(), 0);
}

#[test]
fn auto_refresh_with_observable_predicate2() {
    let item1 = ActivableItem::new(false);

    let items = ObservableCollection::<Rc<ActivableItem>>::new();
    items.add(item1.clone());

    let filter_subject = BehaviorSubject::new(Rc::new(|_: &Rc<ActivableItem>| false) as ActivablePredicate);

    let obs_list_derived = items
        .to_observable_change_set()
        .auto_refresh(prop!(ActivableItem => activated))
        .filter_dynamic(filter_subject.as_observable())
        .as_observable_list();

    // Default filter predicate denies all items
    // The binding collection should stay empty, until the predicate changes
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(true);
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(false);
    assert_eq!(obs_list_derived.count(), 0);

    item1.set_activated(true);
    assert_eq!(obs_list_derived.count(), 0);

    // Changing predicate, all "Activated" items should added to the binding collection
    filter_subject.on_next(Rc::new(|i: &Rc<ActivableItem>| i.activated()) as ActivablePredicate);

    assert_eq!(obs_list_derived.count(), 1);
    assert_equivalent(&obs_list_derived.items(), &[item1.clone()]);

    // Changing property value multiple times
    item1.set_activated(false);
    item1.set_activated(true);
    item1.set_activated(false);
    item1.set_activated(true);
    assert_eq!(obs_list_derived.count(), 1);
    assert_equivalent(&obs_list_derived.items(), &[item1.clone()]);
}
