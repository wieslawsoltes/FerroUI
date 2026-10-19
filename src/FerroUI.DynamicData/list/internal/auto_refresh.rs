//! `AutoRefresh<TObject, TAny>` (`List/Internal/AutoRefresh.cs`).

use crate::list::{clone_changes, Change, ChangeSet, ListChangeReason};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The source's changes, with a refresh of an item, at its index, whenever the sequence
/// `reevaluator` makes for it produces a value (`AutoRefresh.Run`): one change set per value, or
/// the refreshes of each `buffer` together, on `scheduler` (upstream's
/// `GlobalConfig.DefaultScheduler` when none, which is [`SchedulerDefaults::default_scheduler`]
/// here).
pub(crate) fn auto_refresh<T: Clone + PartialEq + 'static, A: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    reevaluator: Rc<dyn Fn(&T) -> Rc<dyn IObservable<A>>>,
    buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        // clone all items so we can look up the index when a change has been made
        let all_items: Rc<RefCell<Vec<T>>> = Rc::new(RefCell::new(Vec::new()));
        let target = all_items.clone();
        let shared = source.do_(move |changes: &ChangeSet<T>| clone_changes(&mut *target.borrow_mut(), changes)).publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<T>>> = shared.clone();

        // monitor each item observable and create change
        let reevaluator = reevaluator.clone();
        let item_has_changed = merge_list_item_observables(
            shared_source.clone(),
            Rc::new(move |item: &T| {
                let item = item.clone();
                reevaluator(&item).select(move |_| item.clone())
            }),
        );

        // create a change set, either buffered or one item at the time
        let items_changed: Rc<dyn IObservable<Vec<T>>> = match buffer {
            None => item_has_changed.select(|item: T| vec![item]),
            Some(buffer) => item_has_changed
                .buffer_time_on(buffer, scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler))
                .where_(|items: &Vec<T>| !items.is_empty()),
        };

        // catch all the indices of items which have been refreshed
        let requires_refresh = items_changed.select(move |items: Vec<T>| {
            let all_items = all_items.borrow();
            index_of_many(&all_items, &items, |item, index| Change::new(ListChangeReason::Refresh, item.clone(), Some(index))).into_iter().collect::<ChangeSet<T>>()
        });

        // publish refreshes and underlying changes
        let publisher = shared_source.merge(requires_refresh).subscribe(observer);
        let connection = shared.connect();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([publisher, connection]);
        composite
    })
}

/// The results of `result_selector` for the elements of `source` equal to each item of
/// `items_to_find`, with their indexes, in the order of `items_to_find` (`EnumerableEx.IndexOfMany`,
/// a join of the items to find with the indexed source).
pub(crate) fn index_of_many<T: PartialEq, R>(source: &[T], items_to_find: &[T], result_selector: impl Fn(&T, usize) -> R) -> Vec<R> {
    let mut results = Vec::new();
    for item in items_to_find {
        for (index, element) in source.iter().enumerate() {
            if element == item {
                results.push(result_selector(element, index));
            }
        }
    }
    results
}

/// The values of the sequences `selector` makes for the items of the source, merged
/// (`MergeMany<T, TDestination>.Run`, the port of `List/Internal/MergeMany.cs`): the sequence of an
/// item is subscribed while the item is in the list.
pub(crate) fn merge_list_item_observables<T: Clone + PartialEq + 'static, D: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<D>>>,
) -> Rc<dyn IObservable<D>> {
    super::merge_many::merge_many(source, selector)
}
