//! `PersonObs` (`Domain/PersonObs.cs`).

use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_system::IObservable;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A person whose age is a sequence (`PersonObs`): the age is a `BehaviorSubject<int>` that
/// [`set_age`](Self::set_age) pushes to. Equal by name; held in `Rc`.
pub struct PersonObs {
    name: String,
    gender: String,
    parent_name: String,
    age: Rc<BehaviorSubject<i32>>,
}

/// An equality comparer of these people (`IEqualityComparer<PersonObs>`).
pub type PersonObsEqualityComparer = fn(&Rc<PersonObs>, &Rc<PersonObs>) -> bool;

impl PersonObs {
    /// `new PersonObs(string name, int age)`: the gender `F`, no parent.
    pub fn new(name: impl Into<String>, age: i32) -> Rc<Self> {
        Self::with_all(name, age, "F", "")
    }

    /// `new PersonObs(string name, int age, string gender, string? parentName)`.
    pub fn with_all(name: impl Into<String>, age: i32, gender: impl Into<String>, parent_name: impl Into<String>) -> Rc<Self> {
        Rc::new(Self { name: name.into(), gender: gender.into(), parent_name: parent_name.into(), age: BehaviorSubject::new(age) })
    }

    /// `new PersonObs(string firstname, string lastname, int age, string gender, string?
    /// parentName)`.
    pub fn from_names_with(first_name: &str, last_name: &str, age: i32, gender: impl Into<String>, parent_name: impl Into<String>) -> Rc<Self> {
        Self::with_all(format!("{first_name} {last_name}"), age, gender, parent_name)
    }

    fn current_age(&self) -> i32 {
        self.age.value().expect("the age of a person is not disposed")
    }

    /// People whose current ages are equal (`AgeComparer`).
    pub fn age_comparer() -> PersonObsEqualityComparer {
        |x, y| Rc::ptr_eq(x, y) || x.current_age() == y.current_age()
    }

    /// People of the same name and gender whose ages are the same subject
    /// (`NameAgeGenderComparer`). As upstream, the subjects of the ages compare by reference, so
    /// two people are equal by it only when they are the same person.
    pub fn name_age_gender_comparer() -> PersonObsEqualityComparer {
        |x, y| Rc::ptr_eq(x, y) || (x.name == y.name && Rc::ptr_eq(&x.age, &y.age) && x.gender == y.gender)
    }

    /// The age, now and whenever it changes (`Age`).
    pub fn age(&self) -> Rc<dyn IObservable<i32>> {
        self.age.as_observable()
    }

    /// Pushes a new age (`SetAge`).
    pub fn set_age(&self, age: i32) {
        ferroui_system::IObserver::on_next(&*self.age, age);
    }

    /// The gender (`Gender`).
    pub fn gender(&self) -> String {
        self.gender.clone()
    }

    /// The key: the name (`Key`).
    pub fn key(&self) -> String {
        self.name.clone()
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The name of the parent (`ParentName`).
    pub fn parent_name(&self) -> String {
        self.parent_name.clone()
    }
}

impl PartialEq for PersonObs {
    /// People of the same name are equal (`Equals(PersonObs)`).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.name == other.name
    }
}

impl Eq for PersonObs {}

impl Hash for PersonObs {
    /// `GetHashCode`: the hash of the name.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl fmt::Display for PersonObs {
    /// `ToString`: `{Name}. {Age}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.age.value() {
            Ok(age) => write!(f, "{}. {}", self.name, age),
            Err(_) => write!(f, "{}. ?", self.name),
        }
    }
}

impl fmt::Debug for PersonObs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
