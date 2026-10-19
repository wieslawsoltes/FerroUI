//! `FilterOnObservable<TObject, TKey>` (`Cache/Internal/FilterOnObservable.cs`).

use super::auto_refresh::auto_refresh;
use super::filter::filter_static;
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::IObservable;
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// An item with the latest value of its filter (`FilterOnObservable.FilterProxy`).
struct FilterProxy<T> {
    value: T,
    passes_filter: Rc<Cell<bool>>,
    observable: Rc<dyn IObservable<bool>>,
}

impl<T> FilterProxy<T> {
    /// The values of the filter that differ from the previous one, recorded as they pass
    /// (`FilterObservable`).
    fn filter_observable(&self) -> Rc<dyn IObservable<bool>> {
        let passes_filter = self.passes_filter.clone();
        self.observable.distinct_until_changed().do_(move |filter_value: &bool| passes_filter.set(*filter_value))
    }
}

/// The items whose sequence of `filter_factory` produced `true` last
/// (`FilterOnObservable.Run`): `Transform` to proxies, `AutoRefreshOnObservable` on their filters,
/// `Filter` on their latest values, `TransformImmutable` back to the items.
pub(crate) fn filter_on_observable<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    filter_factory: Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<bool>>>,
    buffer: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    let proxies = transform_to_proxies(source, filter_factory);
    let refreshed = auto_refresh(proxies, Rc::new(|proxy: &Rc<FilterProxy<T>>, _: &K| proxy.filter_observable()), buffer, scheduler);
    let filtered = filter_static(refreshed, Rc::new(|proxy: &Rc<FilterProxy<T>>| proxy.passes_filter.get()), true);
    filtered.select(|changes: ChangeSet<Rc<FilterProxy<T>>, K>| {
        changes
            .iter()
            .map(|change| {
                Change::with_previous(
                    change.reason(),
                    change.key().clone(),
                    change.current().value.clone(),
                    change.previous().map(|previous| previous.value.clone()),
                    change.current_index(),
                    change.previous_index(),
                )
            })
            .collect::<ChangeSet<T, K>>()
    })
}

/// `Transform((val, key) => new FilterProxy(val, filterFactory(val, key)))`: a proxy for each
/// added or updated item, the proxy of its key for a refresh or a removal.
fn transform_to_proxies<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    filter_factory: Rc<dyn Fn(&T, &K) -> Rc<dyn IObservable<bool>>>,
) -> Rc<dyn IObservable<ChangeSet<Rc<FilterProxy<T>>, K>>> {
    Observable::defer(move || {
        let proxies = RefCell::new(ChangeAwareCache::<Rc<FilterProxy<T>>, K>::new());
        let filter_factory = filter_factory.clone();
        source
            .select(move |changes: ChangeSet<T, K>| {
                let mut proxies = proxies.borrow_mut();
                for change in &changes {
                    match change.reason() {
                        ChangeReason::Add | ChangeReason::Update => {
                            let proxy = Rc::new(FilterProxy {
                                value: change.current().clone(),
                                passes_filter: Rc::new(Cell::new(false)),
                                observable: filter_factory(change.current(), change.key()),
                            });
                            proxies.add_or_update(proxy, change.key().clone());
                        }
                        ChangeReason::Remove => proxies.remove(change.key()),
                        ChangeReason::Refresh => proxies.refresh(change.key()),
                        ChangeReason::Moved => {}
                    }
                }
                proxies.capture_changes()
            })
    })
}
