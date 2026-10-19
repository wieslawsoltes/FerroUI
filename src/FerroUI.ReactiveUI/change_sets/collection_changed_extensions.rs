//! `CollectionChangedExtensions` (`ReactiveUI.Core/ChangeSets/CollectionChangedExtensions.cs`).

use super::CollectionChanged;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::collections::specialized::{CollectionChange, INotifyCollectionChanged};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// The `CollectionChanged` event of a collection as an observable (`CollectionChangedExtensions`).
///
/// It is implemented for the collections that are shared handles (`Clone`), such as
/// `ObservableCollection<T>`, since each notification carries the collection.
pub trait CollectionChangedExtensions: Sized + 'static {
    /// The notifications of the `CollectionChanged` event of the collection, while subscribed
    /// (`ObserveCollectionChanges`).
    fn observe_collection_changes(&self) -> Rc<dyn IObservable<CollectionChanged<Self>>>;
}

impl<S: INotifyCollectionChanged + Clone + 'static> CollectionChangedExtensions for S {
    fn observe_collection_changes(&self) -> Rc<dyn IObservable<CollectionChanged<Self>>> {
        Rc::new(CollectionChangedObservable { source: self.clone() })
    }
}

/// `CollectionChangedObservable`: forwards each event of the collection while subscribed.
struct CollectionChangedObservable<S> {
    source: S,
}

impl<S: INotifyCollectionChanged + Clone + 'static> IObservable<CollectionChanged<S>> for CollectionChangedObservable<S> {
    fn subscribe(&self, observer: Rc<dyn IObserver<CollectionChanged<S>>>) -> Rc<dyn IDisposable> {
        // `Subscription`: hooks the event, and unhooks it when disposed.
        let sender = self.source.clone();
        let token = self.source.collection_changed().add(Rc::new(move |change: &CollectionChange| {
            observer.on_next(CollectionChanged::new(sender.clone(), *change));
        }));
        let source = self.source.clone();
        Disposable::create(move || {
            let _ = source.collection_changed().remove(token);
        })
    }
}
