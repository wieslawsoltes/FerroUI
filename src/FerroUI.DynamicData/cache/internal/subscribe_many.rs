//! `SubscribeMany<TObject, TKey>` (`Cache/Internal/SubscribeMany.cs`): a subscription for each item,
//! made when the item is added and disposed when it is removed or replaced, or when the
//! subscription ends.

use super::{dispose_many, transform_items};
use crate::cache::ChangeSet;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt, Observer};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// `SubscribeMany.Run`: the source is shared; one subscription makes the subscription of each
/// item (`Transform(subscriptionFactory).DisposeMany()`), another forwards the change sets.
pub(crate) fn subscribe_many<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    subscription_factory: Rc<dyn Fn(&T, &K) -> Rc<dyn IDisposable>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let published = source.publish();
        let shared: Rc<dyn IObservable<ChangeSet<T, K>>> = published.clone();
        let error_observer = observer.clone();
        let subscriptions = dispose_many(transform_items(shared.clone(), subscription_factory.clone()), Rc::new(|item: &Rc<dyn IDisposable>| Some(item.clone())))
            .subscribe(Observer::create_all(|_| {}, move |error| error_observer.on_error(error), || {}));
        let forwarded = shared.subscribe(observer);
        let connection = published.connect();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscriptions, forwarded, connection]);
        composite
    })
}
