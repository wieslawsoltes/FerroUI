//! `Person` and `Color` (`Domain/Person.cs`).

use super::{AnimalFamily, FakeEnum, Guid};
use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A favorite color (`Color`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Color {
    #[default]
    NotSpecified,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
}

impl FakeEnum for Color {
    fn values() -> &'static [Self] {
        &[Color::NotSpecified, Color::Red, Color::Orange, Color::Yellow, Color::Green, Color::Blue, Color::Indigo, Color::Violet]
    }
}

/// The person of most tests (`Person`): a name, which is its key and its identity, a gender, and
/// properties that raise `PropertyChanged` (`Age`, `AgeNullable`, `FavoriteColor`, `PetType`,
/// `ParentName`). People are held in `Rc`; the constructors return one.
pub struct Person {
    notify: AbstractNotifyPropertyChanged,
    name: String,
    gender: String,
    unique_key: String,
    age: Cell<i32>,
    age_nullable: Cell<Option<i32>>,
    favorite_color: Cell<Color>,
    pet_type: Cell<AnimalFamily>,
    parent_name: RefCell<String>,
}

/// An equality comparer of people (`IEqualityComparer<Person>`).
pub type PersonEqualityComparer = fn(&Rc<Person>, &Rc<Person>) -> bool;

impl Person {
    fn create(name: String, age: i32, age_nullable: Option<i32>, gender: String, parent_name: String, unique_key: Option<String>) -> Rc<Self> {
        Rc::new(Self {
            notify: AbstractNotifyPropertyChanged::new(),
            name,
            gender,
            unique_key: unique_key.unwrap_or_else(|| Guid::new_guid().to_string_b()),
            age: Cell::new(age),
            age_nullable: Cell::new(age_nullable),
            favorite_color: Cell::new(Color::NotSpecified),
            pet_type: Cell::new(AnimalFamily::Mammal),
            parent_name: RefCell::new(parent_name),
        })
    }

    /// `new Person()`: the name `unknown`, the age 0 and the gender `none`. Upstream's
    /// `new Person() { Name = .. }` is [`with_all`](Self::with_all)`(name, 0, "none", parent_name)`.
    pub fn unknown() -> Rc<Self> {
        Self::with_all("unknown", 0, "none", "")
    }

    /// `new Person(string name, int age)`: the gender `F`, no parent.
    pub fn new(name: impl Into<String>, age: i32) -> Rc<Self> {
        Self::with_all(name, age, "F", "")
    }

    /// `new Person(string name, int age, string gender)`.
    pub fn with_gender(name: impl Into<String>, age: i32, gender: impl Into<String>) -> Rc<Self> {
        Self::with_all(name, age, gender, "")
    }

    /// `new Person(string name, int age, parentName: ..)`: the gender `F`.
    pub fn with_parent_name(name: impl Into<String>, age: i32, parent_name: impl Into<String>) -> Rc<Self> {
        Self::with_all(name, age, "F", parent_name)
    }

    /// `new Person(string name, int age, string gender, string? parentName)`; a `null` parent name
    /// is the empty one.
    pub fn with_all(name: impl Into<String>, age: i32, gender: impl Into<String>, parent_name: impl Into<String>) -> Rc<Self> {
        Self::create(name.into(), age, None, gender.into(), parent_name.into(), None)
    }

    /// `new Person(string firstname, string lastname, int age)`: the name is both, separated by a
    /// space.
    pub fn from_names(first_name: &str, last_name: &str, age: i32) -> Rc<Self> {
        Self::with_all(format!("{first_name} {last_name}"), age, "F", "")
    }

    /// `new Person(string firstname, string lastname, int age, string gender, string? parentName)`.
    pub fn from_names_with(first_name: &str, last_name: &str, age: i32, gender: impl Into<String>, parent_name: impl Into<String>) -> Rc<Self> {
        Self::with_all(format!("{first_name} {last_name}"), age, gender, parent_name)
    }

    /// `new Person(string name, int? age)`: the age goes to `AgeNullable`, and `Age` is 0.
    pub fn with_nullable_age(name: impl Into<String>, age: Option<i32>) -> Rc<Self> {
        Self::with_nullable_age_all(name, age, "F", "")
    }

    /// `new Person(string name, int? age, string gender, string? parentName)`.
    pub fn with_nullable_age_all(name: impl Into<String>, age: Option<i32>, gender: impl Into<String>, parent_name: impl Into<String>) -> Rc<Self> {
        Self::create(name.into(), 0, age, gender.into(), parent_name.into(), None)
    }

    /// A copy of `source_data` with the unique key and parent name of `source_id`
    /// (`CloneUniqueId`). As upstream, the age of `source_data` goes to `AgeNullable` of the copy
    /// (the constructor that takes `int?`), and its `Age` is 0.
    pub fn clone_unique_id(source_data: &Person, source_id: &Person) -> Rc<Self> {
        let person = Self::create(
            source_data.name.clone(),
            0,
            Some(source_data.age()),
            source_data.gender.clone(),
            source_id.parent_name(),
            Some(source_id.unique_key.clone()),
        );
        person.set_favorite_color(source_data.favorite_color());
        person.set_pet_type(source_data.pet_type());
        person
    }

    /// People of the same age are equal (`AgeComparer`).
    pub fn age_comparer() -> PersonEqualityComparer {
        |x, y| Rc::ptr_eq(x, y) || x.age.get() == y.age.get()
    }

    /// People of the same name, age and gender are equal (`NameAgeGenderComparer`).
    pub fn name_age_gender_comparer() -> PersonEqualityComparer {
        |x, y| Rc::ptr_eq(x, y) || (x.name == y.name && x.age.get() == y.age.get() && x.gender == y.gender)
    }

    /// By age, then by name (`DefaultComparer`: `SortExpressionComparer<Person>.Ascending(p =>
    /// p.Age).ThenByAscending(p => p.Name)`).
    ///
    /// Deviation (DEVIATIONS.md, DynamicData): names compare by their characters (ordinal), where
    /// the default comparer of strings upstream compares by the current culture.
    pub fn default_comparer() -> Rc<dyn Fn(&Rc<Person>, &Rc<Person>) -> Ordering> {
        Rc::new(|x: &Rc<Person>, y: &Rc<Person>| x.compare_to(y))
    }

    /// By age, then by name (`CompareTo(Person)`).
    pub fn compare_to(&self, other: &Person) -> Ordering {
        self.age.get().cmp(&other.age.get()).then_with(|| self.name.cmp(&other.name))
    }

    /// The age (`Age`).
    pub fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets the age, raising `Age` when it changes.
    pub fn set_age(&self, value: i32) {
        self.notify.set_and_raise(&self.age, value, "Age");
    }

    /// The age that may be unknown (`AgeNullable`).
    pub fn age_nullable(&self) -> Option<i32> {
        self.age_nullable.get()
    }

    /// Sets the age that may be unknown, raising `AgeNullable` when it changes.
    pub fn set_age_nullable(&self, value: Option<i32>) {
        self.notify.set_and_raise(&self.age_nullable, value, "AgeNullable");
    }

    /// The favorite color (`FavoriteColor`).
    pub fn favorite_color(&self) -> Color {
        self.favorite_color.get()
    }

    /// Sets the favorite color, raising `FavoriteColor` when it changes.
    pub fn set_favorite_color(&self, value: Color) {
        self.notify.set_and_raise(&self.favorite_color, value, "FavoriteColor");
    }

    /// The family of the person's pet (`PetType`).
    pub fn pet_type(&self) -> AnimalFamily {
        self.pet_type.get()
    }

    /// Sets the family of the person's pet, raising `PetType` when it changes.
    pub fn set_pet_type(&self, value: AnimalFamily) {
        self.notify.set_and_raise(&self.pet_type, value, "PetType");
    }

    /// The gender (`Gender`).
    pub fn gender(&self) -> String {
        self.gender.clone()
    }

    /// The key: the name (`Key`).
    pub fn key(&self) -> String {
        self.name.clone()
    }

    /// A key unique to this person, which [`clone_unique_id`](Self::clone_unique_id) copies
    /// (`UniqueKey`).
    pub fn unique_key(&self) -> String {
        self.unique_key.clone()
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The name of the parent (`ParentName`).
    pub fn parent_name(&self) -> String {
        self.parent_name.borrow().clone()
    }

    /// Sets the name of the parent, raising `ParentName` when it changes.
    pub fn set_parent_name(&self, value: impl Into<String>) {
        self.notify.set_and_raise(&self.parent_name, value.into(), "ParentName");
    }

    /// The part that raises `PropertyChanged` (`AbstractNotifyPropertyChanged`).
    pub fn notify(&self) -> &AbstractNotifyPropertyChanged {
        &self.notify
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for Person {
    /// People of the same name are equal (`Equals(Person)`).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || self.name == other.name
    }
}

impl Eq for Person {}

impl Hash for Person {
    /// `GetHashCode`: the hash of the name.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for Person {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Person {
    /// `IComparable<Person>`: by age, then by name. As upstream, two people of the same name and
    /// different ages are equal and do not compare as equal.
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_to(other)
    }
}

impl fmt::Display for Person {
    /// `ToString`: `{Name}. {Age}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}. {}", self.name, self.age.get())
    }
}

impl fmt::Debug for Person {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
