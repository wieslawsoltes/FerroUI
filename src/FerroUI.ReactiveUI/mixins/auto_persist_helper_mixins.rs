//! `AutoPersistHelperMixins` (`ReactiveUI.Shared/Mixins/AutoPersistHelperMixins.cs`): AutoPersist
//! for the objects and collections whose type declares its persisted properties, and
//! `ActOnEveryObject`.

use super::auto_persist_helper_mixins_persistence::{act_on_every_object, start_auto_persist, track_per_item_persistence, AutoPersistMetadata, PersistFunction, DEFAULT_AUTO_PERSIST_INTERVAL};
use crate::IReactiveObject;
use ferroui_reactive::Observable;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;
use std::time::Duration;

/// A reactive object whose type declares which of its properties are persisted: the counterpart
/// of `[DataContract]` with `[DataMember]` on the properties, which upstream reads by reflection.
///
/// ```
/// use ferroui_reactiveui::{reactive_object, reactive_properties, AutoPersistMetadata, DataContract, ReactiveObject};
///
/// reactive_properties! {
///     #[derive(Default)]
///     pub struct Settings {
///         reactive: ReactiveObject,
///         #[reactive]
///         user_name: String,
///         #[reactive]
///         window_title: String,
///     }
/// }
/// reactive_object!(Settings);
///
/// impl DataContract for Settings {
///     fn data_members() -> &'static [&'static str] {
///         &["UserName"]
///     }
/// }
///
/// let metadata = AutoPersistMetadata::for_type::<Settings>();
/// assert!(metadata.has_data_contract());
/// assert!(metadata.persistable_property_names().contains("UserName"));
/// assert!(!metadata.persistable_property_names().contains("WindowTitle"));
/// ```
// Deviation (DEVIATIONS.md, ReactiveUI): the persisted properties are declared explicitly, by
// name, where upstream reads `[DataContract]` and `[DataMember]` by reflection; a type without
// the contract does not implement the trait.
pub trait DataContract: IReactiveObject {
    /// The names of the persisted properties, in PascalCase (the properties with `[DataMember]`).
    fn data_members() -> &'static [&'static str];
}

/// AutoPersist: saves a reactive object when its persisted properties change, throttled on an
/// interval (the extension members of `IReactiveObject` in `AutoPersistHelperMixins`), on its
/// shared handle.
///
/// The persisted properties are the ones [`DataContract`] declares; the methods of
/// [`AutoPersistMetadataMixins`](crate::AutoPersistMetadataMixins) take them as explicit metadata.
/// Watching starts on [`RxSchedulers::main_thread_scheduler`](crate::RxSchedulers::main_thread_scheduler),
/// and the throttle runs on [`RxSchedulers::task_pool_scheduler`](crate::RxSchedulers::task_pool_scheduler).
/// An object that changes all the time may never be saved.
///
/// | Upstream | Here |
/// |---|---|
/// | `AutoPersist(Func)` | [`auto_persist`](Self::auto_persist) |
/// | `AutoPersist(Func, TimeSpan?)` | [`auto_persist_with_interval`](Self::auto_persist_with_interval) |
/// | `AutoPersist<TDontCare>(Func, IObservable<TDontCare>, TimeSpan?)`, and without the interval | [`auto_persist_with`](Self::auto_persist_with) |
// Deviation (DEVIATIONS.md, ReactiveUI): the overloads have one name per set of arguments (Rust has
// no overloading); the interval is an `Option<Duration>` (`TimeSpan?`), and the overloads that read
// `[DataMember]` by reflection take the type's [`DataContract`] instead.
pub trait AutoPersistHelperMixins<T: IReactiveObject> {
    /// Saves the object with `do_persist` after its persisted properties change, with the default
    /// interval of three seconds (`AutoPersist(Func<T, IObservable<Unit>>)`).
    ///
    /// Returns the disposable that stops watching.
    fn auto_persist(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static) -> Rc<dyn IDisposable>
    where
        T: DataContract;

    /// [`auto_persist`](Self::auto_persist), throttled on `interval` (the default one when `None`)
    /// (`AutoPersist(Func, TimeSpan?)`).
    fn auto_persist_with_interval(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, interval: Option<Duration>) -> Rc<dyn IDisposable>
    where
        T: DataContract;

    /// [`auto_persist`](Self::auto_persist), throttled on `interval`, and also whenever
    /// `manual_save_signal` ticks, changed or not (`AutoPersist<TDontCare>(Func,
    /// IObservable<TDontCare>, TimeSpan?)`).
    fn auto_persist_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>
    where
        T: DataContract;
}

impl<T: IReactiveObject> AutoPersistHelperMixins<T> for Rc<T> {
    fn auto_persist(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        self.auto_persist_with_interval(do_persist, None)
    }

    fn auto_persist_with_interval(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, interval: Option<Duration>) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        self.auto_persist_with(do_persist, Observable::never::<()>(), interval)
    }

    fn auto_persist_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        let metadata = AutoPersistMetadata::for_type::<T>();
        let do_persist: PersistFunction<T> = Rc::new(do_persist);
        start_auto_persist(self.clone(), do_persist, metadata, manual_save_signal, interval.unwrap_or(DEFAULT_AUTO_PERSIST_INTERVAL))
    }
}

/// AutoPersist for each item of a collection, and `ActOnEveryObject` (the extension members of
/// `ObservableCollection<TItem>` in `AutoPersistHelperMixins`): an item is saved while it is in
/// the collection, and no longer after it was removed.
///
/// | Upstream | Here |
/// |---|---|
/// | `AutoPersistCollection(Func)` | [`auto_persist_collection`](Self::auto_persist_collection) |
/// | `AutoPersistCollection(Func, TimeSpan?)` | [`auto_persist_collection_with_interval`](Self::auto_persist_collection_with_interval) |
/// | `AutoPersistCollection<TDontCare>(Func, IObservable<TDontCare>, TimeSpan?)`, and without the interval | [`auto_persist_collection_with`](Self::auto_persist_collection_with) |
/// | `ActOnEveryObject(Action<TItem>, Action<TItem>)` | [`act_on_every_object`](Self::act_on_every_object) |
///
/// The overloads with explicit metadata are those of
/// [`AutoPersistCollectionMetadataMixins`](crate::AutoPersistCollectionMetadataMixins).
// Deviation (DEVIATIONS.md, ReactiveUI): the overloads of `ReadOnlyObservableCollection<TItem>`
// are not ported (`ferroui-system` has no such collection), and `ActOnEveryObject` of an
// `IObservable<IReactiveChangeSet<TItem>>` is not ported in this module (it needs the change sets
// of `ChangeSets/`); `ActOnEveryObject` of a collection follows its typed change notification.
pub trait AutoPersistCollectionMixins<T: IReactiveObject> {
    /// Persists each item with the default interval (`AutoPersistCollection(Func)`).
    fn auto_persist_collection(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static) -> Rc<dyn IDisposable>
    where
        T: DataContract;

    /// Persists each item, throttled on `interval` (`AutoPersistCollection(Func, TimeSpan?)`).
    fn auto_persist_collection_with_interval(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, interval: Option<Duration>) -> Rc<dyn IDisposable>
    where
        T: DataContract;

    /// Persists each item, throttled on `interval`, and whenever `manual_save_signal` ticks
    /// (`AutoPersistCollection<TDontCare>(Func, IObservable<TDontCare>, TimeSpan?)`).
    fn auto_persist_collection_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>
    where
        T: DataContract;

    /// Calls `on_add` for each item now and for each item added, and `on_remove` for each item
    /// removed; a reset removes the items there were and adds the items there are. Disposing the
    /// result stops watching and calls `on_remove` for every item (`ActOnEveryObject`).
    fn act_on_every_object(&self, on_add: impl Fn(Rc<T>) + 'static, on_remove: impl Fn(Rc<T>) + 'static) -> Rc<dyn IDisposable>;
}

impl<T: IReactiveObject + PartialEq> AutoPersistCollectionMixins<T> for ObservableCollection<Rc<T>> {
    fn auto_persist_collection(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        self.auto_persist_collection_with_interval(do_persist, None)
    }

    fn auto_persist_collection_with_interval(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, interval: Option<Duration>) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        self.auto_persist_collection_with(do_persist, Observable::never::<()>(), interval)
    }

    fn auto_persist_collection_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>
    where
        T: DataContract,
    {
        let do_persist: PersistFunction<T> = Rc::new(do_persist);
        let metadata = AutoPersistMetadata::for_type::<T>();
        track_per_item_persistence(self, move |item: Rc<T>| {
            start_auto_persist(item, do_persist.clone(), metadata.clone(), manual_save_signal.clone(), interval.unwrap_or(DEFAULT_AUTO_PERSIST_INTERVAL))
        })
    }

    fn act_on_every_object(&self, on_add: impl Fn(Rc<T>) + 'static, on_remove: impl Fn(Rc<T>) + 'static) -> Rc<dyn IDisposable> {
        act_on_every_object(self, on_add, on_remove)
    }
}
