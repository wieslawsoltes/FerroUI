//! `GroupOnProperty<TObject, TKey, TGroup>` (`Cache/Internal/GroupOnProperty.cs`).

use super::group_on::group_on;
use crate::cache::{ChangeSet, GroupChangeSet, ObservableCacheEx};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// The groups of the items of `source` by the value of `group_selector_key`, evaluated again when
/// that value changes on an item, after `throttle` without another change when one is given
/// (`GroupOnProperty.Run`).
pub(crate) fn group_on_property<T, K, G>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    group_selector_key: PropertyPath<T, G>,
    throttle: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<GroupChangeSet<T, K, G>>>
where
    T: Clone + 'static,
    K: Eq + Hash + Clone + 'static,
    G: Eq + Hash + Clone + 'static,
{
    let group_selector = group_selector_key.getter();
    source.publish_with(move |shared: Rc<dyn IObservable<ChangeSet<T, K>>>| {
        // Monitor the changes of the property.
        let mut regrouper: Rc<dyn IObservable<()>> = shared.when_value_changed_with(group_selector_key.clone(), false).select(|_| ());

        // Throttle them, when asked.
        if let Some(throttle) = throttle {
            regrouper = regrouper.throttle_on(throttle, scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler));
        }

        // A change of the property evaluates the groups again.
        group_on(shared, group_selector.clone(), Some(regrouper))
    })
}
