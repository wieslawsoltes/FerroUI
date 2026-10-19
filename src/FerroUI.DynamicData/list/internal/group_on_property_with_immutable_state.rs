//! `GroupOnPropertyWithImmutableState<TObject, TGroup>`
//! (`List/Internal/GroupOnPropertyWithImmutableState.cs`).

use super::group_on_immutable::group_on_immutable;
use crate::list::{ChangeSet, ImmutableGroup, ObservableListEx};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// The states of the groups of the items of `source` by the value of `group_selector_key`,
/// evaluated again when that value changes on an item, after `throttle` without another change
/// when one is given (`GroupOnPropertyWithImmutableState.Run`).
pub(crate) fn group_on_property_with_immutable_state<T, G>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    group_selector_key: PropertyPath<T, G>,
    throttle: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<ImmutableGroup<T, G>>>>
where
    T: Clone + PartialEq + 'static,
    G: Eq + Hash + Clone + 'static,
{
    let group_selector = group_selector_key.getter();
    source.publish_with(move |shared: Rc<dyn IObservable<ChangeSet<T>>>| {
        // Monitor the changes of the property.
        let mut regrouper: Rc<dyn IObservable<()>> = shared.when_value_changed_with(group_selector_key.clone(), false).select(|_| ());

        // Throttle them, when asked.
        if let Some(throttle) = throttle {
            regrouper = regrouper.throttle_on(throttle, scheduler.clone().unwrap_or_else(SchedulerDefaults::default_scheduler));
        }

        // A change of the property evaluates the groups again.
        group_on_immutable(shared, group_selector.clone(), Some(regrouper))
    })
}
