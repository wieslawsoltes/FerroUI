//! `PersonEmpKey` and `PersonEmployment` (`Domain/PersonEmployment.cs`).

use std::fmt;
use std::rc::Rc;

/// The key of an employment: the name of the person and the company (`PersonEmpKey`, a struct).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PersonEmpKey {
    name: String,
    company: String,
}

impl PersonEmpKey {
    /// `new PersonEmpKey(string name, string company)`.
    pub fn new(name: impl Into<String>, company: impl Into<String>) -> Self {
        Self { name: name.into(), company: company.into() }
    }

    /// `new PersonEmpKey(PersonEmployment)`.
    pub fn from_employment(person_employment: &PersonEmployment) -> Self {
        Self::new(person_employment.name.clone(), person_employment.company.clone())
    }
}

/// The employment of a person by a company (`PersonEmployment`, `IKey<PersonEmpKey>`). Equal by
/// name and company; held in `Rc`.
#[derive(PartialEq, Eq, Hash)]
pub struct PersonEmployment {
    name: String,
    company: String,
    key: PersonEmpKey,
}

impl PersonEmployment {
    /// `new PersonEmployment(string name, string company)`.
    pub fn new(name: impl Into<String>, company: impl Into<String>) -> Rc<Self> {
        let (name, company) = (name.into(), company.into());
        let key = PersonEmpKey::new(name.clone(), company.clone());
        Rc::new(Self { name, company, key })
    }

    /// The company (`Company`).
    pub fn company(&self) -> String {
        self.company.clone()
    }

    /// The key (`Key`).
    pub fn key(&self) -> PersonEmpKey {
        self.key.clone()
    }

    /// The name of the person (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for PersonEmployment {
    /// `ToString`: `Name: {Name}, Company: {Company}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name: {}, Company: {}", self.name, self.company)
    }
}

impl fmt::Debug for PersonEmployment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
