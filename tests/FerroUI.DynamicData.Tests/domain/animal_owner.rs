//! `AnimalOwner` (`Domain/AnimalOwner.cs`).

use super::{Animal, Guid};
use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_dynamic_data::cache::{ObservableCache, ObservableCacheEx};
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The owner of animals (`AnimalOwner`): a name, an identifier, a list of animals, and whether it
/// is included in results (`IncludeInResults`, which raises `PropertyChanged`). Owners are held in
/// `Rc`, and two owners are equal only when they are the same one.
///
/// Not ported: `ObservableCollection` (`Animals.Connect().Bind(out var collection)`), which waits
/// for `Bind` and its target; only `TransformManyAsyncFixture` reads it.
pub struct AnimalOwner {
    notify: AbstractNotifyPropertyChanged,
    id: Guid,
    name: String,
    animals: SourceList<Rc<Animal>>,
    include_in_results: Cell<bool>,
    observable_cache: RefCell<Option<ObservableCache<Rc<Animal>, i32>>>,
}

impl AnimalOwner {
    /// `new AnimalOwner(name)`: a new identifier, included in results.
    pub fn new(name: impl Into<String>) -> Rc<Self> {
        Self::with_id(name, None, true)
    }

    /// `new AnimalOwner(name, id, include)`: a new identifier when `id` is `None`.
    pub fn with_id(name: impl Into<String>, id: Option<Guid>, include: bool) -> Rc<Self> {
        Rc::new(Self {
            notify: AbstractNotifyPropertyChanged::new(),
            id: id.unwrap_or_else(Guid::new_guid),
            name: name.into(),
            animals: SourceList::new(),
            include_in_results: Cell::new(include),
            observable_cache: RefCell::new(None),
        })
    }

    /// The identifier (`Id`).
    pub fn id(&self) -> Guid {
        self.id
    }

    /// The name (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The animals (`Animals`).
    pub fn animals(&self) -> &SourceList<Rc<Animal>> {
        &self.animals
    }

    /// The animals keyed by their identifiers, made on first use
    /// (`ObservableCache`: `Animals.Connect().AddKey(a => a.Id).AsObservableCache()`).
    pub fn observable_cache(&self) -> ObservableCache<Rc<Animal>, i32> {
        if let Some(cache) = self.observable_cache.borrow().as_ref() {
            return cache.clone();
        }
        let cache = self.animals.connect().add_key(|animal: &Rc<Animal>| animal.id()).as_observable_cache();
        *self.observable_cache.borrow_mut() = Some(cache.clone());
        cache
    }

    /// Whether the owner is included in results (`IncludeInResults`).
    pub fn include_in_results(&self) -> bool {
        self.include_in_results.get()
    }

    /// Sets whether the owner is included in results, raising `IncludeInResults` when it changes.
    pub fn set_include_in_results(&self, value: bool) {
        self.notify.set_and_raise(&self.include_in_results, value, "IncludeInResults");
    }

    /// The part that raises `PropertyChanged` (`AbstractNotifyPropertyChanged`).
    pub fn notify(&self) -> &AbstractNotifyPropertyChanged {
        &self.notify
    }
}

impl IDisposable for AnimalOwner {
    /// Disposes the cache of the animals and the list (`Dispose`).
    fn dispose(&self) {
        let cache = self.observable_cache.borrow().clone();
        if let Some(cache) = cache {
            cache.dispose();
        }
        self.animals.dispose();
    }
}

impl INotifyPropertyChanged for AnimalOwner {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for AnimalOwner {
    /// The same owner (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for AnimalOwner {}

impl Hash for AnimalOwner {
    /// The hash of the identity of the owner (`object.GetHashCode`).
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

impl fmt::Display for AnimalOwner {
    /// `ToString`: `{Name} [{Animals.Count} Animals] ({Id:B})`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{} Animals] ({})", self.name, self.animals.count(), self.id.to_string_b())
    }
}

impl fmt::Debug for AnimalOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
