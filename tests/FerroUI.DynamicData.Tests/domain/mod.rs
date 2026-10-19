//! The objects of the tests (`DynamicData.Tests/Domain`), one module per upstream file.
//!
//! The conventions of the port:
//! - **Classes are held in `Rc`.** Their constructors return `Rc<Self>` (`Person::new("Name", 20)`);
//!   upstream's `struct`s and `record`s (`PersonEmpKey`, `PersonWithAgeGroup`, `Pet`) are values.
//! - **Equality is upstream's.** A class that overrides `Equals` compares the same way (`Person` by
//!   name, `MarketPrice` by its parts); one that does not is equal only to itself (`Animal`,
//!   `AnimalOwner`, `Market`). Equality comparers are functions (`Person::age_comparer()`), and
//!   comparers are `Rc<dyn Fn(&T, &T) -> Ordering>` (`Person::default_comparer()`).
//! - **Properties are getters and setters.** A mutable property is a `Cell` or `RefCell` with
//!   `age()` and `set_age(..)`; the setter of an object that raises `PropertyChanged` (through an
//!   embedded `AbstractNotifyPropertyChanged`) raises the PascalCase name (`Age`), the name that
//!   `prop!(Person => age)` derives.
//! - **`ToString` is `Display`**, and `Debug` prints the same.
//! - **Random data.** Bogus is replaced by the seeded generator of [`fakers`]: `Fakers.Person`
//!   is `fakers::person()`, `UseSeed(n)` is `use_seed(n)`, `Generate(n)` is `generate(n)`.

mod animal;
mod animal_owner;
pub mod fakers;
mod guid;
mod market;
mod market_price;
mod parent_and_children;
mod parent_child;
mod person;
mod person_employment;
mod person_obs;
mod person_with_children;
mod person_with_employment;
mod person_with_friends;
mod person_with_gender;
mod person_with_relations;
mod pet;
mod random_person_generator;
mod self_observing_person;

pub use animal::{Animal, AnimalEqualityComparer, AnimalFamily};
pub use animal_owner::AnimalOwner;
pub use fakers::{Commerce, FakeEnum, FakePerson, Faker, FakerContext, Hacker, Randomizer};
pub use guid::Guid;
pub use market::{FixedMarket, IMarket, Market, PriceChangeSets};
pub use market_price::{MarketPrice, MarketPriceComparer, MarketPriceEqualityComparer};
pub use parent_and_children::ParentAndChildren;
pub use parent_child::ParentChild;
pub use person::{Color, Person, PersonEqualityComparer};
pub use person_employment::{PersonEmpKey, PersonEmployment};
pub use person_obs::{PersonObs, PersonObsEqualityComparer};
pub use person_with_children::PersonWithChildren;
pub use person_with_employment::{EmploymentCache, PersonWithEmployment};
pub use person_with_friends::PersonWithFriends;
pub use person_with_gender::{PersonWithAgeGroup, PersonWithGender};
pub use person_with_relations::PersonWithRelations;
pub use pet::Pet;
pub use random_person_generator::RandomPersonGenerator;
pub use self_observing_person::SelfObservingPerson;

/// Compares two numbers as `Double.CompareTo` does: a `NaN` is equal to a `NaN` and less than any
/// other number.
pub fn compare_f64(x: f64, y: f64) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match x.partial_cmp(&y) {
        Some(ordering) => ordering,
        None => match (x.is_nan(), y.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            _ => Ordering::Greater,
        },
    }
}
