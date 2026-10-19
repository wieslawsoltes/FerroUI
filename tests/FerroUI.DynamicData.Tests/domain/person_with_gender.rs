//! `PersonWithAgeGroup` and `PersonWithGender` (`Domain/PersonWithGender.cs`).

use super::Person;
use std::fmt;
use std::rc::Rc;

/// A person and their age group (`record PersonWithAgeGroup(Person Person, string AgeGroup)`): a
/// value, equal by its parts.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PersonWithAgeGroup {
    /// The person (`Person`).
    pub person: Rc<Person>,
    /// The age group (`AgeGroup`).
    pub age_group: String,
}

impl PersonWithAgeGroup {
    /// `new PersonWithAgeGroup(Person, string)`.
    pub fn new(person: Rc<Person>, age_group: impl Into<String>) -> Self {
        Self { person, age_group: age_group.into() }
    }
}

/// A name, an age and a gender (`PersonWithGender`): equal by all three; held in `Rc`.
#[derive(PartialEq, Eq, Hash)]
pub struct PersonWithGender {
    name: String,
    age: i32,
    gender: String,
}

impl PersonWithGender {
    /// `new PersonWithGender(Person person, string gender)`: the name and age of the person.
    pub fn from_person(person: &Person, gender: impl Into<String>) -> Rc<Self> {
        Self::new(person.name(), person.age(), gender)
    }

    /// `new PersonWithGender(string name, int age, string gender)`.
    pub fn new(name: impl Into<String>, age: i32, gender: impl Into<String>) -> Rc<Self> {
        Rc::new(Self { name: name.into(), age, gender: gender.into() })
    }

    /// The age (`Age`).
    pub fn age(&self) -> i32 {
        self.age
    }

    /// The gender (`Gender`).
    pub fn gender(&self) -> String {
        self.gender.clone()
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for PersonWithGender {
    /// `ToString`: `{Name}. {Age} ({Gender})`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {} ({})", self.name, self.age, self.gender)
    }
}

impl fmt::Debug for PersonWithGender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
