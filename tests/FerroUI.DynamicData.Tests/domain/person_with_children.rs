//! `PersonWithChildren` (`Domain/PersonWithChildren.cs`).

use super::Person;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// A person and their relations (`PersonWithChildren`, `IKey<string>`): the age can be set without
/// a notification. Held in `Rc`; two are equal only when they are the same one.
pub struct PersonWithChildren {
    name: String,
    age: Cell<i32>,
    relations: Vec<Rc<Person>>,
}

impl PersonWithChildren {
    /// `new PersonWithChildren(string name, int age)`: no relations.
    pub fn new(name: impl Into<String>, age: i32) -> Rc<Self> {
        Self::with_relations(name, age, Vec::new())
    }

    /// `new PersonWithChildren(string name, int age, IEnumerable<Person> relations)`.
    pub fn with_relations(name: impl Into<String>, age: i32, relations: Vec<Rc<Person>>) -> Rc<Self> {
        Rc::new(Self { name: name.into(), age: Cell::new(age), relations })
    }

    /// The age (`Age`).
    pub fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets the age, without a notification.
    pub fn set_age(&self, value: i32) {
        self.age.set(value);
    }

    /// The key: the name (`Key`).
    pub fn key(&self) -> String {
        self.name.clone()
    }

    /// The name (`KeyValue`).
    pub fn key_value(&self) -> String {
        self.name.clone()
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The relations (`Relations`).
    pub fn relations(&self) -> Vec<Rc<Person>> {
        self.relations.clone()
    }
}

impl PartialEq for PersonWithChildren {
    /// The same person (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for PersonWithChildren {
    /// `ToString`: `{Name}. {Age}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {}", self.name, self.age.get())
    }
}

impl fmt::Debug for PersonWithChildren {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
