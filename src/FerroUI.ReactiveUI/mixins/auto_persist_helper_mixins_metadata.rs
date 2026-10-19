//! `AutoPersistHelperMixins.Metadata` (`ReactiveUI.Shared/Mixins/AutoPersistHelperMixins.Metadata.cs`):
//! AutoPersist with explicit metadata.

use super::auto_persist_helper_mixins_persistence::{require_data_contract, start_auto_persist, AutoPersistMetadata, PersistFunction, DEFAULT_AUTO_PERSIST_INTERVAL};
use crate::IReactiveObject;
use ferroui_reactive::Observable;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;
use std::time::Duration;

/// AutoPersist with explicit metadata of persistence (the metadata-based extension members of
/// `IReactiveObject` in `AutoPersistHelperMixins.Metadata`), on the shared handle of an object.
///
/// | Upstream | Here |
/// |---|---|
/// | `AutoPersist(Func, AutoPersistMetadata)` | [`auto_persist_metadata`](Self::auto_persist_metadata) |
/// | `AutoPersist(Func, AutoPersistMetadata, TimeSpan?)` | [`auto_persist_metadata_with_interval`](Self::auto_persist_metadata_with_interval) |
/// | `AutoPersist<TDontCare>(Func, IObservable<TDontCare>, AutoPersistMetadata, TimeSpan?)`, and without the interval | [`auto_persist_metadata_with`](Self::auto_persist_metadata_with) |
pub trait AutoPersistMetadataMixins<T: IReactiveObject> {
    /// Saves the object after a property of `metadata` changes, with the default interval
    /// (`AutoPersist(Func, AutoPersistMetadata)`).
    fn auto_persist_metadata(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, metadata: Rc<AutoPersistMetadata>) -> Rc<dyn IDisposable>;

    /// [`auto_persist_metadata`](Self::auto_persist_metadata), throttled on `interval`
    /// (`AutoPersist(Func, AutoPersistMetadata, TimeSpan?)`).
    fn auto_persist_metadata_with_interval(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>;

    /// [`auto_persist_metadata`](Self::auto_persist_metadata), throttled on `interval`, and also
    /// whenever `manual_save_signal` ticks (`AutoPersist<TDontCare>(Func, IObservable<TDontCare>,
    /// AutoPersistMetadata, TimeSpan?)`).
    ///
    /// Panics when the metadata is not of a data contract (`ArgumentException`).
    fn auto_persist_metadata_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable>;
}

impl<T: IReactiveObject> AutoPersistMetadataMixins<T> for Rc<T> {
    fn auto_persist_metadata(&self, do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static, metadata: Rc<AutoPersistMetadata>) -> Rc<dyn IDisposable> {
        self.auto_persist_metadata_with_interval(do_persist, metadata, None)
    }

    fn auto_persist_metadata_with_interval(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable> {
        self.auto_persist_metadata_with(do_persist, Observable::never::<()>(), metadata, interval)
    }

    fn auto_persist_metadata_with<S: 'static>(
        &self,
        do_persist: impl Fn(Rc<T>) -> Rc<dyn IObservable<()>> + 'static,
        manual_save_signal: Rc<dyn IObservable<S>>,
        metadata: Rc<AutoPersistMetadata>,
        interval: Option<Duration>,
    ) -> Rc<dyn IDisposable> {
        require_data_contract(&metadata);
        let do_persist: PersistFunction<T> = Rc::new(do_persist);
        start_auto_persist(self.clone(), do_persist, metadata, manual_save_signal, interval.unwrap_or(DEFAULT_AUTO_PERSIST_INTERVAL))
    }
}
