//! `AutoRefresh<TObject, TKey, TAny>` (`Cache/Internal/AutoRefresh.cs`), with the merge of two
//! sequences through a delivery queue that it is built on (`DeliveryQueueMerge`,
//! `Internal/DeliveryQueueMergeExtensions.cs`).

use crate::cache::{Change, ChangeReason, ChangeSet};
use crate::internal::DeliveryQueue;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// The source's changes, with a refresh of an item whenever the sequence `reevaluator` makes for
/// it produces a value (`AutoRefresh.Run`): one refresh per change set, or the refreshes of each
/// `buffer` together, on `scheduler` (upstream's `GlobalConfig.DefaultScheduler` when none, which
/// is [`SchedulerDefaults::default_scheduler`] here).
pub(crate) fn auto_refresh<T: Clone + 'static, K: Eq + Hash + Clone + 'static, A: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    reevaluator: Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<A>>>,
    buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    let scheduler = scheduler.unwrap_or_else(SchedulerDefaults::default_scheduler);
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let shared = source.publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<T, K>>> = shared.clone();

        // monitor each item observable and create change
        let reevaluator = reevaluator.clone();
        let changes = merge_item_observables(
            shared_source.clone(),
            Rc::new(move |item: &T, key: &K| {
                let (item, key) = (item.clone(), key.clone());
                reevaluator(&item, &key).select(move |_| Change::new(ChangeReason::Refresh, key.clone(), item.clone()))
            }),
        );

        // create a change set, either buffered or one item at the time
        let refresh_changes: Rc<dyn IObservable<ChangeSet<T, K>>> = match buffer {
            None => changes.select(|change: Change<T, K>| ChangeSet::from(vec![change])),
            Some(buffer) => changes
                .buffer_time_on(buffer, scheduler.clone())
                .where_(|items: &Vec<Change<T, K>>| !items.is_empty())
                .select(|items: Vec<Change<T, K>>| ChangeSet::from(items)),
        };

        // publish refreshes and underlying changes
        let publisher = delivery_queue_merge(shared_source, refresh_changes).subscribe(observer);
        let connection = shared.connect();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([publisher, connection]);
        composite
    })
}

/// The values of the sequences `selector` makes for the items of the source, merged
/// (`MergeMany<TObject, TKey, TDestination>.Run`, the port of `Cache/Internal/MergeMany.cs`): the
/// sequence of an item is subscribed while the item is in the cache.
pub(crate) fn merge_item_observables<T: Clone + 'static, K: Eq + Hash + Clone + 'static, D: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    selector: Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<D>>>,
) -> Rc<dyn IObservable<D>> {
    super::merge_many::merge_many(source, selector)
}

/// `first` and `second` merged through a [`DeliveryQueue`] (`DeliveryQueueMerge`): it ends after
/// both end, the first error ends it, and the sources are subscribed in order.
fn delivery_queue_merge<T: 'static>(first: Rc<dyn IObservable<T>>, second: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    Observable::create(move |observer: Rc<dyn IObserver<T>>| {
        let queue = DeliveryQueue::new(observer);
        let remaining_sources = Rc::new(Cell::new(2usize));
        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::with_capacity(2);
        for source in [&first, &second] {
            let (next, error, completed, remaining) = (queue.clone(), queue.clone(), queue.clone(), remaining_sources.clone());
            subscriptions.push(source.subscribe_all(
                move |value| next.acquire().enqueue_next(value),
                move |e| error.acquire().enqueue_error(e),
                move || {
                    remaining.set(remaining.get().saturating_sub(1));
                    if remaining.get() == 0 {
                        completed.acquire().enqueue_completed();
                    }
                },
            ));
        }
        // Subscriptions first, so any terminal notification produced during the disposal cascade
        // still flows through the still-active queue; the queue last as cleanup.
        Disposable::create(move || {
            for subscription in subscriptions {
                subscription.dispose();
            }
            queue.dispose();
        })
    })
}
