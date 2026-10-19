//! `AutoPersistHelperMixins.Persistence` (`ReactiveUI.Shared/Mixins/AutoPersistHelperMixins.Persistence.cs`):
//! the metadata of persistence, the persistence of the items of a collection with explicit
//! metadata, and the driver that saves an object.

use super::auto_persist_helper_mixins::DataContract;
use crate::{IReactiveNotifyPropertyChanged, IReactiveObject, ReactivePropertyChangedEventArgs, RxSchedulers};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::{Disposable, ICancelable, SerialDisposable, SingleAssignmentDisposable};
use ferroui_reactive::Observer;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_system::{IDisposable, IObservable};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The interval of the throttle of a save when none is given (`DefaultAutoPersistIntervalSeconds`).
pub(crate) const DEFAULT_AUTO_PERSIST_INTERVAL: Duration = Duration::from_secs(3);

/// The function that persists an object; the observable completes once it is saved.
pub type PersistFunction<T> = Rc<dyn Fn(Rc<T>) -> Rc<dyn IObservable<()>>>;

/// The function that gives the metadata of persistence of an item of a collection
/// (`Func<TItem, AutoPersistMetadata>`).
pub type MetadataProvider<T> = Rc<dyn Fn(&Rc<T>) -> Rc<AutoPersistMetadata>>;

/// What AutoPersist needs to know of an object: whether it is a data contract, and which of its
/// properties are persisted (`AutoPersistHelperMixins.AutoPersistMetadata`).
///
/// Upstream reads it by reflection from `[DataContract]` and `[DataMember]`; here it is given
/// explicitly, or declared once per type with [`DataContract`] and read with
/// [`for_type`](Self::for_type).
// Deviation (DEVIATIONS.md, ReactiveUI): the persisted properties are listed explicitly (by name,
// or by `PropertyPath`), where upstream reads `[DataMember]` by reflection.
#[derive(Clone, PartialEq, Eq)]
pub struct AutoPersistMetadata {
    has_data_contract: bool,
    persistable_property_names: HashSet<String>,
}

impl fmt::Debug for AutoPersistMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AutoPersistMetadata {{ has_data_contract: {}, properties: {} }}", self.has_data_contract, self.persistable_property_names.len())
    }
}

ferroui_system::ferro_thread_local! {
    /// The metadata of the types that declare a [`DataContract`], by type (`PersistMetadataHolder<T>`).
    static METADATA_BY_TYPE: RefCell<HashMap<TypeId, Rc<AutoPersistMetadata>>> = RefCell::new(HashMap::new());
}

impl AutoPersistMetadata {
    /// The metadata of an object: whether it is a data contract, and the names of its persisted
    /// properties, in PascalCase (`AutoPersistMetadata(bool, ISet<string>)`).
    pub fn new<S: Into<String>>(has_data_contract: bool, persistable_property_names: impl IntoIterator<Item = S>) -> Self {
        Self { has_data_contract, persistable_property_names: persistable_property_names.into_iter().map(Into::into).collect() }
    }

    /// The metadata of a data contract whose persisted properties are `persistable_property_names`.
    pub fn with_properties<S: Into<String>>(persistable_property_names: impl IntoIterator<Item = S>) -> Self {
        Self::new(true, persistable_property_names)
    }

    /// Adds the property at the start of `path` to the persisted properties, for a list written
    /// with [`prop!`](ferroui_system::prop) rather than with names.
    pub fn with_path<S: 'static, V: 'static>(mut self, path: &ferroui_system::component_model::PropertyPath<S, V>) -> Self {
        if let Some(name) = path.names().first() {
            self.persistable_property_names.insert(name.clone());
        }
        self
    }

    /// Whether the object is a data contract (`HasDataContract`).
    pub fn has_data_contract(&self) -> bool {
        self.has_data_contract
    }

    /// The names of the persisted properties (`PersistablePropertyNames`).
    pub fn persistable_property_names(&self) -> &HashSet<String> {
        &self.persistable_property_names
    }

    /// The metadata a type declares with [`DataContract`], created once per type and thread
    /// (`AutoPersistHelperMixins.CreateMetadata<T>()`).
    pub fn for_type<T: DataContract>() -> Rc<AutoPersistMetadata> {
        METADATA_BY_TYPE.with(|metadata| {
            metadata
                .borrow_mut()
                .entry(TypeId::of::<T>())
                .or_insert_with(|| Rc::new(AutoPersistMetadata::with_properties(T::data_members().iter().copied())))
                .clone()
        })
    }

    /// A provider that gives the metadata of [`for_type`](Self::for_type) for every item
    /// (`AutoPersistHelperMixins.CreateMetadataProvider<TItem>()`).
    pub fn provider_for_type<T: DataContract>() -> MetadataProvider<T> {
        let metadata = Self::for_type::<T>();
        Rc::new(move |_: &Rc<T>| metadata.clone())
    }
}

/// Panics unless the metadata is of a data contract (`ArgumentException`).
pub(crate) fn require_data_contract(metadata: &AutoPersistMetadata) {
    if !metadata.has_data_contract() {
        panic!("AutoPersist can only be applied to objects with [DataContract]");
    }
}

/// The persistence of the items of a collection with explicit metadata (the extension members of
/// `TCollection` in `AutoPersistHelperMixins.Persistence`): each item in the collection is saved
/// when it changes, and an item removed from it is no longer watched.
///
/// The collection is an [`ObservableCollection`] of shared reactive objects.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream takes any `INotifyCollectionChanged` that is an
// `IEnumerable<TItem>`; the change notification of `ferroui-system` that carries the items is the
// typed one of `ObservableCollection<T>`, which is the collection taken here.
pub trait AutoPersistCollectionMetadataMixins<T: IReactiveObject> {
    /// Persists each item with `metadata`, with the default interval
    /// (`AutoPersistCollection(Func, AutoPersistMetadata)`).
    fn auto_persist_collection_metadata(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, metadata: Rc<AutoPersistMetadata>) -> Rc<dyn IDisposable>;

    /// Persists each item with `metadata`, throttled on `interval`
    /// (`AutoPersistCollection(Func, AutoPersistMetadata, TimeSpan?)`).
    fn auto_persist_collection_metadata_with_interval(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>;

    /// Persists each item with `metadata`, throttled on `interval`, and whenever
    /// `manual_save_signal` ticks (`AutoPersistCollection<TDontCare>(Func, IObservable<TDontCare>,
    /// AutoPersistMetadata, TimeSpan?)` and the overload without the interval).
    ///
    /// Panics when the metadata is not of a data contract (`ArgumentException`).
    fn auto_persist_collection_metadata_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>;

    /// Persists each item with the metadata `metadata_provider` gives for it, throttled on
    /// `interval`, and whenever `manual_save_signal` ticks (`AutoPersistCollection<TDontCare>(Func,
    /// IObservable<TDontCare>, Func<TItem, AutoPersistMetadata>, TimeSpan?)` and the overload
    /// without the interval).
    fn auto_persist_collection_metadata_provider<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata_provider: MetadataProvider<T>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>;
}

impl<T: IReactiveObject + PartialEq> AutoPersistCollectionMetadataMixins<T> for ObservableCollection<Rc<T>> {
    fn auto_persist_collection_metadata(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, metadata: Rc<AutoPersistMetadata>) -> Rc<dyn IDisposable> {
        self.auto_persist_collection_metadata_with_interval(do_persist, metadata, None)
    }

    fn auto_persist_collection_metadata_with_interval(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable> {
        self.auto_persist_collection_metadata_with(do_persist, ferroui_reactive::Observable::never::<()>(), metadata, interval)
    }

    fn auto_persist_collection_metadata_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable> {
        require_data_contract(&metadata);
        let do_persist: PersistFunction<T> = Rc::new(do_persist);
        track_per_item_persistence(self, move |item: Rc<T>| {
            start_auto_persist(item, do_persist.clone(), metadata.clone(), manual_save_signal.clone(), interval.unwrap_or(DEFAULT_AUTO_PERSIST_INTERVAL))
        })
    }

    fn auto_persist_collection_metadata_provider<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata_provider: MetadataProvider<T>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable> {
        let do_persist: PersistFunction<T> = Rc::new(do_persist);
        track_per_item_persistence(self, move |item: Rc<T>| {
            let metadata = metadata_provider(&item);
            require_data_contract(&metadata);
            start_auto_persist(item, do_persist.clone(), metadata, manual_save_signal.clone(), interval.unwrap_or(DEFAULT_AUTO_PERSIST_INTERVAL))
        })
    }
}

/// Starts persisting `source` on the main-thread scheduler, unless the returned handle is disposed
/// first (`StartAutoPersist`).
pub(crate) fn start_auto_persist<T: IReactiveObject, S: 'static>(
    source: Rc<T>,
    do_persist: PersistFunction<T>,
    metadata: Rc<AutoPersistMetadata>,
    manual_save_signal: Rc<dyn IObservable<S>>,
    interval: Duration,
) -> Rc<dyn IDisposable> {
    let handle = SingleAssignmentDisposable::new();
    let started = handle.clone();
    let _ = RxSchedulers::main_thread_scheduler().schedule(Box::new(move |_| {
        if started.is_disposed() {
            return Disposable::empty();
        }
        let driver = AutoPersistDriver::start(source, do_persist, metadata, manual_save_signal, interval, RxSchedulers::task_pool_scheduler());
        started.set_disposable(Some(driver as Rc<dyn IDisposable>));
        Disposable::empty()
    }));
    handle
}

/// Calls `on_add` for each item of `collection`, now and when it is added, and `on_remove` when an
/// item is removed; a reset removes the items there were and adds the items there are. Disposing
/// the result stops watching and calls `on_remove` for each item (`ActOnEveryObject`).
pub(crate) fn act_on_every_object<T: Clone + PartialEq + 'static>(
    collection: &ObservableCollection<T>,
    on_add: impl Fn(T) + 'static,
    on_remove: impl Fn(T) + 'static,
) -> Rc<dyn IDisposable> {
    let on_add: Rc<dyn Fn(T)> = Rc::new(on_add);
    let on_remove: Rc<dyn Fn(T)> = Rc::new(on_remove);
    let items = collection.to_vec();
    for item in items.iter().cloned() {
        on_add(item);
    }
    let known = RefCell::new(items);
    let observed = collection.clone();
    let add = on_add.clone();
    let remove = on_remove.clone();
    let token = collection.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<'_, T>| {
        match args.action {
            NotifyCollectionChangedAction::Add => {
                for item in args.new_items.iter().cloned() {
                    add(item);
                }
            }
            NotifyCollectionChangedAction::Remove => {
                for item in args.old_items.iter().cloned() {
                    remove(item);
                }
            }
            NotifyCollectionChangedAction::Replace => {
                for item in args.old_items.iter().cloned() {
                    remove(item);
                }
                for item in args.new_items.iter().cloned() {
                    add(item);
                }
            }
            NotifyCollectionChangedAction::Move => {}
            NotifyCollectionChangedAction::Reset => {
                let previous = known.borrow().clone();
                for item in previous {
                    remove(item);
                }
                for item in observed.to_vec() {
                    add(item);
                }
            }
        }
        *known.borrow_mut() = observed.to_vec();
    }));
    let collection = collection.clone();
    Disposable::create(move || {
        collection.remove_collection_changed(token);
        for item in collection.to_vec() {
            on_remove(item);
        }
    })
}

/// Starts tracking each item of the collection once, and stops tracking it when it is removed
/// (`TrackPerItemPersistence`).
pub(crate) fn track_per_item_persistence<T: 'static>(
    collection: &ObservableCollection<Rc<T>>,
    start_tracking: impl Fn(Rc<T>) -> Rc<dyn IDisposable> + 'static,
) -> Rc<dyn IDisposable>
where
    Rc<T>: PartialEq,
{
    let trackers: Rc<RefCell<Vec<(Rc<T>, Rc<dyn IDisposable>)>>> = Rc::new(RefCell::new(Vec::new()));
    let added = trackers.clone();
    let removed = trackers.clone();
    let subscription = act_on_every_object(
        collection,
        move |item: Rc<T>| {
            if added.borrow().iter().any(|(tracked, _)| Rc::ptr_eq(tracked, &item)) {
                return;
            }
            let tracker = start_tracking(item.clone());
            added.borrow_mut().push((item, tracker));
        },
        move |item: Rc<T>| dispose_and_remove(&removed, &item),
    );
    Disposable::create(move || {
        subscription.dispose();
        let tracked: Vec<(Rc<T>, Rc<dyn IDisposable>)> = trackers.borrow_mut().drain(..).collect();
        for (_, tracker) in tracked {
            tracker.dispose();
        }
    })
}

/// Stops tracking `item`, when it is tracked (`DisposeAndRemove`).
fn dispose_and_remove<T>(trackers: &RefCell<Vec<(Rc<T>, Rc<dyn IDisposable>)>>, item: &Rc<T>) {
    let position = trackers.borrow().iter().position(|(tracked, _)| Rc::ptr_eq(tracked, item));
    let Some(position) = position else {
        return;
    };
    let (_, tracker) = trackers.borrow_mut().remove(position);
    tracker.dispose();
}

/// Saves an object after its persisted properties change or the manual signal ticks, throttled on
/// the interval (`AutoPersistDriver<T, TDontCare>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): as upstream, the subscriptions keep the driver alive, so
/// that the returned handle may be dropped (`_ = x.AutoPersist(...)`); the driver holds its object
/// weakly, so that the object's change subject, which holds the driver, does not keep the object
/// alive, and the driver stops itself once the object is gone.
struct AutoPersistDriver<T> {
    target: Weak<T>,
    do_persist: PersistFunction<T>,
    metadata: Rc<AutoPersistMetadata>,
    interval: Duration,
    scheduler: Rc<dyn IScheduler>,
    /// The pending save; scheduling another one disposes it (`_debounce`).
    debounce: Rc<SerialDisposable>,
    change_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    manual_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    persist_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl<T: IReactiveObject> AutoPersistDriver<T> {
    /// A driver that watches `target` and `manual_save_signal` (`AutoPersistDriver(...)`).
    fn start<S: 'static>(
        target: Rc<T>,
        do_persist: PersistFunction<T>,
        metadata: Rc<AutoPersistMetadata>,
        manual_save_signal: Rc<dyn IObservable<S>>,
        interval: Duration,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<Self> {
        let driver = Rc::new(Self {
            target: Rc::downgrade(&target),
            do_persist,
            metadata,
            interval,
            scheduler,
            debounce: SerialDisposable::new(),
            change_subscription: RefCell::new(None),
            manual_subscription: RefCell::new(None),
            persist_subscription: RefCell::new(None),
            disposed: Cell::new(false),
        });
        let observed = driver.clone();
        let change_subscription = target.changed().subscribe(Observer::create_all(
            move |args: ReactivePropertyChangedEventArgs<Rc<T>>| {
                observed.clone().on_property_changed(&args.property_name);
            },
            |_| {},
            || {},
        ));
        *driver.change_subscription.borrow_mut() = Some(change_subscription);
        let signalled = driver.clone();
        let manual_subscription = manual_save_signal.subscribe(Observer::create_all(
            move |_: S| {
                signalled.clone().request_save();
            },
            |_| {},
            || {},
        ));
        *driver.manual_subscription.borrow_mut() = Some(manual_subscription);
        driver
    }

    /// Saves when a persisted property changed (`OnPropertyChanged`).
    fn on_property_changed(self: Rc<Self>, property_name: &str) {
        if !self.metadata.persistable_property_names().contains(property_name) {
            return;
        }
        self.request_save();
    }

    /// Schedules a save after the interval, replacing the one pending (`RequestSave`).
    fn request_save(self: Rc<Self>) {
        if self.disposed.get() {
            return;
        }
        if self.target.strong_count() == 0 {
            self.dispose();
            return;
        }
        let weak = Rc::downgrade(&self);
        let pending = self.scheduler.schedule_after(
            self.interval,
            Box::new(move |_| {
                if let Some(driver) = weak.upgrade() {
                    driver.run_persist();
                }
                Disposable::empty()
            }),
        );
        self.debounce.set_disposable(Some(pending));
    }

    /// Saves the object (`RunPersist`).
    fn run_persist(&self) {
        if self.disposed.get() {
            return;
        }
        let Some(target) = self.target.upgrade() else {
            self.dispose();
            return;
        };
        let persist = (self.do_persist)(target);
        let subscription = persist.subscribe(Observer::create_all(|_: ()| {}, |_| {}, || {}));
        if self.disposed.get() {
            subscription.dispose();
            return;
        }
        let previous = self.persist_subscription.borrow_mut().replace(subscription);
        if let Some(previous) = previous {
            previous.dispose();
        }
    }
}

impl<T> IDisposable for AutoPersistDriver<T> {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        for subscription in [self.change_subscription.borrow_mut().take(), self.manual_subscription.borrow_mut().take()].into_iter().flatten() {
            subscription.dispose();
        }
        self.debounce.dispose();
        let persist = self.persist_subscription.borrow_mut().take();
        if let Some(persist) = persist {
            persist.dispose();
        }
    }
}
