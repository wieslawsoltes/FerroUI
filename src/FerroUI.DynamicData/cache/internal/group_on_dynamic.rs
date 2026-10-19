//! `GroupOnDynamic<TObject, TKey, TGroupKey>` (`Cache/Internal/GroupOnDynamic.cs`).

use super::dynamic_grouper::{DynamicGroupSelector, DynamicGrouper};
use super::Cache;
use crate::cache::{ChangeSet, GroupChangeSet, ICache, IQuery};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

/// The groups of the items of `source` by the latest selector of `select_group_observable`,
/// evaluated again for every item when a selector or `regrouper` produces a value
/// (`GroupOnDynamic.Run`). Nothing is grouped until the first selector; the result ends when the
/// source, the selectors and the regrouper have all ended.
pub(crate) fn group_on_dynamic<T, K, G>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    select_group_observable: Rc<dyn IObservable<DynamicGroupSelector<T, K, G>>>,
    regrouper: Option<Rc<dyn IObservable<()>>>,
) -> Rc<dyn IObservable<GroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
{
    let regrouper = regrouper.unwrap_or_else(Observable::empty);
    Observable::create(move |observer: Rc<dyn IObserver<GroupChangeSet<T, K, G>>>| {
        let dynamic_grouper = Rc::new(RefCell::new(DynamicGrouper::new(None)));
        let not_grouped = Rc::new(RefCell::new(Cache::<T, K>::new()));
        let has_selector = Rc::new(Cell::new(false));

        // Shared observables of the three inputs. The `SharedDeliveryQueue` of upstream serializes
        // them; on one thread they are already serial.
        let shared_source = source.publish();
        // `DistinctUntilChanged` of delegates: the same function is not applied twice.
        // Deviation (DEVIATIONS.md, DynamicData): .NET compares delegates by method and target;
        // a selector here is the same when it is the same `Rc`.
        let shared_group_selector =
            select_group_observable.distinct_until_changed_with_comparer(|a: &DynamicGroupSelector<T, K, G>, b: &DynamicGroupSelector<T, K, G>| Rc::ptr_eq(a, b)).publish();
        let shared_regrouper = regrouper.publish();
        let source_values: Rc<dyn IObservable<ChangeSet<T, K>>> = shared_source.clone();
        let selector_values: Rc<dyn IObservable<DynamicGroupSelector<T, K, G>>> = shared_group_selector.clone();
        let regrouper_values: Rc<dyn IObservable<()>> = shared_regrouper.clone();

        // The first selector groups every item seen so far, then marks that there is a selector;
        // the next ones only change the selector.
        let sub_group_selector = {
            let (grouper, not_grouped, has_selector, observer, on_error) = (dynamic_grouper.clone(), not_grouped.clone(), has_selector.clone(), observer.clone(), observer.clone());
            selector_values.subscribe_all(
                move |group_selector: DynamicGroupSelector<T, K, G>| {
                    if has_selector.get() {
                        let emission = grouper.borrow_mut().set_group_selector(group_selector);
                        emission.deliver(&*observer);
                    } else {
                        let initial_values = not_grouped.borrow().key_values();
                        let emission = grouper.borrow_mut().initialize(initial_values, group_selector);
                        emission.deliver(&*observer);
                        has_selector.set(true);
                    }
                },
                move |error| on_error.on_error(error),
                || {},
            )
        };

        // The regrouper is ignored until there is a selector; then it evaluates every item again.
        let sub_regrouper = {
            let (grouper, has_selector, observer, on_error) = (dynamic_grouper.clone(), has_selector.clone(), observer.clone(), observer.clone());
            regrouper_values.subscribe_all(
                move |_| {
                    if has_selector.get() {
                        let emission = grouper.borrow_mut().regroup_all();
                        emission.deliver(&*observer);
                    }
                },
                move |error| on_error.on_error(error),
                || {},
            )
        };

        let sub_changes = {
            let (grouper, not_grouped, has_selector, observer, on_error) = (dynamic_grouper.clone(), not_grouped.clone(), has_selector.clone(), observer.clone(), observer.clone());
            source_values.subscribe_all(
                move |change_set: ChangeSet<T, K>| {
                    if has_selector.get() {
                        let emission = grouper.borrow_mut().process_change_set(&change_set, true);
                        if let Some(emission) = emission {
                            emission.deliver(&*observer);
                        }
                    } else {
                        not_grouped.borrow_mut().clone_changes(&change_set);
                    }
                },
                move |error| on_error.on_error(error),
                || {},
            )
        };

        // The merge joins the ends of the three inputs into one end of the result.
        let sub_on_complete = {
            let (on_error, on_completed) = (observer.clone(), observer.clone());
            source_values
                .select(|_| ())
                .merge(selector_values.select(|_| ()))
                .merge(regrouper_values)
                .ignore_elements()
                .subscribe_all(|_| {}, move |error| on_error.on_error(error), move || on_completed.on_completed())
        };

        let connected_group_selector = shared_group_selector.connect();
        let connected_source = shared_source.connect();
        let connected_regrouper = shared_regrouper.connect();
        Disposable::create(move || {
            connected_group_selector.dispose();
            connected_source.dispose();
            connected_regrouper.dispose();
            let (suspensions, groups) = dynamic_grouper.borrow_mut().take_for_dispose();
            for suspension in suspensions {
                suspension.dispose();
            }
            for group in groups {
                group.dispose();
            }
            sub_changes.dispose();
            sub_group_selector.dispose();
            sub_regrouper.dispose();
            sub_on_complete.dispose();
        })
    })
}

/// `AdaptSelector`: the selectors of the group key of an item as selectors of the item and its
/// key.
pub(crate) fn adapt_group_selector<T: 'static, K: 'static, G: 'static>(
    selectors: Rc<dyn IObservable<Rc<dyn Fn(&T) -> G>>>,
) -> Rc<dyn IObservable<DynamicGroupSelector<T, K, G>>> {
    selectors.select(|selector: Rc<dyn Fn(&T) -> G>| -> DynamicGroupSelector<T, K, G> { Rc::new(move |item: &T, _: &K| selector(item)) })
}
