//! `PersonWithFriends` (`Domain/PersonWithFriends.cs`).

use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// A person and their friends (`PersonWithFriends`, `IKey<string>`): `Age` and `Friends` raise
/// `PropertyChanged`. Held in `Rc`; two are equal only when they are the same one.
pub struct PersonWithFriends {
    notify: AbstractNotifyPropertyChanged,
    name: String,
    age: Cell<i32>,
    friends: RefCell<Vec<Rc<PersonWithFriends>>>,
}

impl PersonWithFriends {
    /// `new PersonWithFriends(string name, int age)`: no friends.
    pub fn new(name: impl Into<String>, age: i32) -> Rc<Self> {
        Self::with_friends(name, age, Vec::new())
    }

    /// `new PersonWithFriends(string name, int age, IEnumerable<PersonWithFriends> friends)`.
    pub fn with_friends(name: impl Into<String>, age: i32, friends: Vec<Rc<PersonWithFriends>>) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), name: name.into(), age: Cell::new(age), friends: RefCell::new(friends) })
    }

    /// The age (`Age`).
    pub fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets the age, raising `Age` when it changes.
    pub fn set_age(&self, value: i32) {
        self.notify.set_and_raise(&self.age, value, "Age");
    }

    /// The friends (`Friends`).
    pub fn friends(&self) -> Vec<Rc<PersonWithFriends>> {
        self.friends.borrow().clone()
    }

    /// Sets the friends, raising `Friends`.
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): upstream compares the enumerables by reference, so
    /// assigning a new array always raises; a `Vec` has no identity, and every assignment raises.
    pub fn set_friends(&self, value: Vec<Rc<PersonWithFriends>>) {
        self.notify.set_and_raise_with(&self.friends, value, |_, _| false, "Friends");
    }

    /// The key: the name (`Key`).
    pub fn key(&self) -> String {
        self.name.clone()
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The part that raises `PropertyChanged` (`AbstractNotifyPropertyChanged`).
    pub fn notify(&self) -> &AbstractNotifyPropertyChanged {
        &self.notify
    }
}

impl INotifyPropertyChanged for PersonWithFriends {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for PersonWithFriends {
    /// The same person (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for PersonWithFriends {
    /// `ToString`: `{Name}. {Age}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {}", self.name, self.age.get())
    }
}

impl fmt::Debug for PersonWithFriends {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
