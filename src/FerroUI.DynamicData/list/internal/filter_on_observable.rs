//! `FilterOnObservable<TObject>` (`List/Internal/FilterOnObservable.cs`).

use super::auto_refresh::merge_list_item_observables;
use super::filter::static_filter;
use super::transformer::transformer;
use crate::list::{clone_changes, Change, ChangeSet, ListChangeReason};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// An item with the latest value of its filter (`ObjWithFilterValue`): equal to another when their
/// items are, whatever the values of their filters, as the filter downstream needs.
#[derive(Clone)]
struct ObjWithFilterValue<T> {
    obj: T,
    filter: bool,
}

impl<T: PartialEq> PartialEq for ObjWithFilterValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.obj == other.obj
    }
}

/// The items whose sequence of `filter` produced `true` last; an item is included until its
/// sequence produces `false` (`FilterOnObservable.Run`). The refreshes the filters cause are not
/// propagated.
pub(crate) fn filter_on_observable<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    filter: Rc<dyn Fn(&T) -> Rc<dyn IObservable<bool>>>,
    buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let all_items: Rc<RefCell<Vec<ObjWithFilterValue<T>>>> = Rc::new(RefCell::new(Vec::new()));
        let target = all_items.clone();
        // we default to true (include all items)
        let shared = transformer(source.clone(), |item: &T, _: Option<&ObjWithFilterValue<T>>, _: usize| Ok(ObjWithFilterValue { obj: item.clone(), filter: true }), false)
            // clone all items so we can look up the index when a change has been made
            .do_(move |changes: &ChangeSet<ObjWithFilterValue<T>>| clone_changes(&mut *target.borrow_mut(), changes))
            .publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<ObjWithFilterValue<T>>>> = shared.clone();

        // monitor each item observable and create change, carry the value of the observable
        // property
        let filter = filter.clone();
        let item_has_changed = merge_list_item_observables(
            shared_source.clone(),
            Rc::new(move |value: &ObjWithFilterValue<T>| {
                let obj = value.obj.clone();
                filter(&obj).select(move |prop: bool| ObjWithFilterValue { obj: obj.clone(), filter: prop })
            }),
        );

        // create a change set, either buffered or one item at the time
        let items_changed: Rc<dyn IObservable<Vec<ObjWithFilterValue<T>>>> = match buffer {
            None => item_has_changed.select(|value: ObjWithFilterValue<T>| vec![value]),
            Some(buffer) => item_has_changed
                .buffer_time_on(buffer, scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler))
                .where_(|values: &Vec<ObjWithFilterValue<T>>| !values.is_empty()),
        };

        // catch all the indices of items which have been refreshed
        let requires_refresh = items_changed.select(move |items: Vec<ObjWithFilterValue<T>>| {
            let all_items = all_items.borrow();
            let mut changes = Vec::new();
            for item in &items {
                for (index, element) in all_items.iter().enumerate() {
                    if element.obj == item.obj {
                        changes.push(Change::new(ListChangeReason::Refresh, item.clone(), Some(index)));
                    }
                }
            }
            ChangeSet::from(changes)
        });

        // publish refreshes and underlying changes
        let filtered = static_filter(shared_source.merge(requires_refresh), Rc::new(|value: &ObjWithFilterValue<T>| value.filter), true);
        let publisher = transformer(filtered, |value: &ObjWithFilterValue<T>, _: Option<&T>, _: usize| Ok(value.obj.clone()), false)
            // suppress refreshes from filter, avoids excessive refresh messages for no-op filter
            // updates
            .select(|changes: ChangeSet<T>| changes.iter().filter(|change| change.reason() != ListChangeReason::Refresh).cloned().collect::<ChangeSet<T>>())
            .where_(|changes: &ChangeSet<T>| changes.count() != 0)
            .subscribe(observer);
        let connection = shared.connect();
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([publisher, connection]);
        composite
    })
}
