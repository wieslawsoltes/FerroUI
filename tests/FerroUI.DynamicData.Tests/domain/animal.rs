//! `Animal` and `AnimalFamily` (`Domain/Animal.cs`).

use super::FakeEnum;
use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering as AtomicOrdering};

/// The family of an animal (`AnimalFamily`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnimalFamily {
    #[default]
    Mammal,
    Reptile,
    Fish,
    Amphibian,
    Bird,
}

impl FakeEnum for AnimalFamily {
    fn values() -> &'static [Self] {
        &[AnimalFamily::Mammal, AnimalFamily::Reptile, AnimalFamily::Fish, AnimalFamily::Amphibian, AnimalFamily::Bird]
    }
}

/// The counter of the identifiers of animals (`s_counter`).
static COUNTER: AtomicI32 = AtomicI32::new(0);

/// An animal (`Animal`): an identifier from a counter of the process, a name, a kind and a family,
/// and whether it is included in results (`IncludeInResults`, which raises `PropertyChanged`).
/// Animals are held in `Rc`, and two animals are equal only when they are the same one, as
/// upstream's class does not override `Equals`.
pub struct Animal {
    notify: AbstractNotifyPropertyChanged,
    id: i32,
    name: String,
    family: AnimalFamily,
    animal_type: String,
    include_in_results: Cell<bool>,
}

/// An equality comparer of animals (`IEqualityComparer<Animal>`).
pub type AnimalEqualityComparer = fn(&Rc<Animal>, &Rc<Animal>) -> bool;

impl Animal {
    /// `new Animal(name, type, family)`: included in results, with the next identifier.
    pub fn new(name: impl Into<String>, animal_type: impl Into<String>, family: AnimalFamily) -> Rc<Self> {
        Self::with_id(name, animal_type, family, true, None)
    }

    /// `new Animal(name, type, family, include)`.
    pub fn with_include(name: impl Into<String>, animal_type: impl Into<String>, family: AnimalFamily, include: bool) -> Rc<Self> {
        Self::with_id(name, animal_type, family, include, None)
    }

    /// `new Animal(name, type, family, include, id)`: the next identifier when `id` is `None`.
    pub fn with_id(name: impl Into<String>, animal_type: impl Into<String>, family: AnimalFamily, include: bool, id: Option<i32>) -> Rc<Self> {
        let id = id.unwrap_or_else(|| COUNTER.fetch_add(1, AtomicOrdering::Relaxed) + 1);
        Rc::new(Self {
            notify: AbstractNotifyPropertyChanged::new(),
            id,
            name: name.into(),
            family,
            animal_type: animal_type.into(),
            include_in_results: Cell::new(include),
        })
    }

    /// The identifier (`Id`).
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The family (`Family`).
    pub fn family(&self) -> AnimalFamily {
        self.family
    }

    /// The kind of animal (`Type`).
    pub fn animal_type(&self) -> String {
        self.animal_type.clone()
    }

    /// `{Name} the {Type}` (`FormalName`).
    pub fn formal_name(&self) -> String {
        format!("{} the {}", self.name, self.animal_type)
    }

    /// Whether the animal is included in results (`IncludeInResults`).
    pub fn include_in_results(&self) -> bool {
        self.include_in_results.get()
    }

    /// Sets whether the animal is included in results, raising `IncludeInResults` when it changes.
    pub fn set_include_in_results(&self, value: bool) {
        self.notify.set_and_raise(&self.include_in_results, value, "IncludeInResults");
    }

    /// By formal name, ignoring case (`NameComparer`: `string.Compare(.., OrdinalIgnoreCase)`).
    pub fn name_comparer() -> Rc<dyn Fn(&Rc<Animal>, &Rc<Animal>) -> Ordering> {
        Rc::new(|x: &Rc<Animal>, y: &Rc<Animal>| x.formal_name().to_uppercase().cmp(&y.formal_name().to_uppercase()))
    }

    /// Animals of the same kind, family and name are equal (`NameTypeCompare`).
    pub fn name_type_compare() -> AnimalEqualityComparer {
        |x, y| x.animal_type == y.animal_type && x.family == y.family && x.name == y.name
    }

    /// Animals of the same identifier are equal (`IdCompare`).
    pub fn id_compare() -> AnimalEqualityComparer {
        |x, y| x.id == y.id
    }

    /// The part that raises `PropertyChanged` (`AbstractNotifyPropertyChanged`).
    pub fn notify(&self) -> &AbstractNotifyPropertyChanged {
        &self.notify
    }
}

impl INotifyPropertyChanged for Animal {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for Animal {
    /// The same animal (`Equals` of `object`: reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Animal {}

impl Hash for Animal {
    /// `GetHashCode`: `HashCode.Combine(Id, Name, Family, Type)`.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.name.hash(state);
        self.family.hash(state);
        self.animal_type.hash(state);
    }
}

impl fmt::Display for Animal {
    /// `ToString`: `{FormalName} ({Family}) [{Id:x4}]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?}) [{:04x}]", self.formal_name(), self.family, self.id)
    }
}

impl fmt::Debug for Animal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
