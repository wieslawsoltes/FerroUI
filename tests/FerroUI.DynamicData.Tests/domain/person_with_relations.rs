//! `PersonWithRelations` (`Domain/PersonWithRelations.cs`).

use super::Pet;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// A person, their relations and their pets (`PersonWithRelations`, `IKey<string>`): the pets can
/// be set without a notification. Held in `Rc`; two are equal only when they are the same one.
pub struct PersonWithRelations {
    name: String,
    age: i32,
    pet: RefCell<Vec<Pet>>,
    relations: Vec<Rc<PersonWithRelations>>,
}

impl PersonWithRelations {
    /// `new PersonWithRelations(string name, int age)`: no relations.
    pub fn new(name: impl Into<String>, age: i32) -> Rc<Self> {
        Self::with_relations(name, age, Vec::new())
    }

    /// `new PersonWithRelations(string name, int age, IEnumerable<PersonWithRelations>
    /// relations)`: no pets.
    pub fn with_relations(name: impl Into<String>, age: i32, relations: Vec<Rc<PersonWithRelations>>) -> Rc<Self> {
        Rc::new(Self { name: name.into(), age, pet: RefCell::new(Vec::new()), relations })
    }

    /// The age (`Age`).
    pub fn age(&self) -> i32 {
        self.age
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

    /// The pets (`Pet`).
    pub fn pet(&self) -> Vec<Pet> {
        self.pet.borrow().clone()
    }

    /// Sets the pets, without a notification.
    pub fn set_pet(&self, value: Vec<Pet>) {
        *self.pet.borrow_mut() = value;
    }

    /// The relations (`Relations`).
    pub fn relations(&self) -> Vec<Rc<PersonWithRelations>> {
        self.relations.clone()
    }
}

impl PartialEq for PersonWithRelations {
    /// The same person (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for PersonWithRelations {
    /// `ToString`: `{Name}. {Age}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {}", self.name, self.age)
    }
}

impl fmt::Debug for PersonWithRelations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
