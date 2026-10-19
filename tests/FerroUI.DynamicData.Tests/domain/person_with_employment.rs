//! `PersonWithEmployment` (`Domain/PersonWithEmployment.cs`).

use super::{PersonEmpKey, PersonEmployment};
use ferroui_dynamic_data::cache::IObservableCache;
use ferroui_system::IDisposable;
use std::fmt;
use std::rc::Rc;

/// The cache of the employments of a person (`IObservableCache<PersonEmployment, PersonEmpKey>`).
pub type EmploymentCache = Rc<dyn IObservableCache<Rc<PersonEmployment>, PersonEmpKey>>;

/// A person with the cache of their employments (`PersonWithEmployment`), made from a group of
/// employments by person. Held in `Rc`; two are equal only when they are the same one.
pub struct PersonWithEmployment {
    person: String,
    employment_data: EmploymentCache,
}

impl PersonWithEmployment {
    /// `new PersonWithEmployment(IGroup<PersonEmployment, PersonEmpKey, string> source)`, from the
    /// key of the group and its cache.
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): the constructor takes the parts of the group it reads
    /// (`source.Key`, `source.Cache`), so that the domain does not depend on the group of `Group`.
    pub fn new(group_key: impl Into<String>, group_cache: EmploymentCache) -> Rc<Self> {
        Rc::new(Self { person: group_key.into(), employment_data: group_cache })
    }

    /// The number of employments (`EmploymentCount`).
    pub fn employment_count(&self) -> usize {
        self.employment_data.count()
    }

    /// The employments (`EmploymentData`).
    pub fn employment_data(&self) -> EmploymentCache {
        self.employment_data.clone()
    }

    /// The name of the person, the key of the group (`Person`).
    pub fn person(&self) -> String {
        self.person.clone()
    }
}

impl IDisposable for PersonWithEmployment {
    /// Disposes the cache of the employments (`Dispose`).
    fn dispose(&self) {
        self.employment_data.dispose();
    }
}

impl PartialEq for PersonWithEmployment {
    /// The same person (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for PersonWithEmployment {
    /// `ToString`: `Person: {Person}. Count {EmploymentCount}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Person: {}. Count {}", self.person, self.employment_count())
    }
}

impl fmt::Debug for PersonWithEmployment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
