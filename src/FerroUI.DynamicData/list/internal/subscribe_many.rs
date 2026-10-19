//! `SubscribeMany<T>` (`List/Internal/SubscribeMany.cs`): a subscription for each item, made when
//! the item is added and disposed when it is removed or replaced, or when the subscription ends.

use super::{dispose_many, transform_items};
use crate::list::ChangeSet;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt, Observer};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// `SubscribeMany.Run`: the source is shared; one subscription makes the subscription of each
/// item (`Transform(subscriptionFactory).DisposeMany()`), another forwards the change sets.
pub(crate) fn subscribe_many<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    subscription_factory: Rc<dyn Fn(&T) -> Rc<dyn IDisposable>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let published = source.publish();
        let shared: Rc<dyn IObservable<ChangeSet<T>>> = published.clone();
        let error_observer = observer.clone();
        let subscriptions = dispose_many(transform_items(shared.clone(), subscription_factory.clone()), Rc::new(|item: &Rc<dyn IDisposable>| item.dispose()))
            .subscribe(Observer::create_all(|_| {}, move |error| error_observer.on_error(error), || {}));
        let forwarded = shared.subscribe(observer);
        let connection = published.connect();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscriptions, forwarded, connection]);
        composite
    })
}
