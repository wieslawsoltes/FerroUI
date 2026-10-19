//! `ObservableListEx` (`List/ObservableListEx*.cs`): the operators of the sequences of change sets
//! of lists.
//!
//! As for caches (`ObservableCacheEx`), each upstream operator is a provided method from the macro
//! of its module in `ops/`, the trait is implemented for the sequences of change sets of any form
//! ([`IChangeSet`]), and an operator whose name is that of an operator of
//! `ferroui_reactive::ObservableExt` takes the suffix `_list` (`and_list`, `cast_list`,
//! `switch_list`, `count_list`, ...). The LINQ-named aliases of `ObservableListAlias` are not ported.

use super::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// The operators of lists (`ObservableListEx`).
pub trait ObservableListEx<T: Clone + PartialEq + 'static> {
    /// The sequence as plain change sets.
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>>;

    super::ops::filter::filter_methods!();

    super::ops::adapt::adapt_methods!();
    super::ops::auto_refresh::auto_refresh_methods!();
    super::ops::auto_refresh_on_observable::auto_refresh_on_observable_methods!();
    super::ops::bind::bind_methods!();
    super::ops::bind_to_observable_list::bind_to_observable_list_methods!();
    super::ops::buffer_if::buffer_if_methods!();
    super::ops::buffer_initial::buffer_initial_methods!();
    super::ops::cast::cast_methods!();
    super::ops::cast_to_object::cast_to_object_methods!();
    super::ops::clone::clone_methods!();
    super::ops::convert::convert_methods!();
    super::ops::defer_until_loaded::defer_until_loaded_methods!();
    super::ops::dispose_many::dispose_many_methods!();
    super::ops::distinct_values::distinct_values_methods!();
    super::ops::filter_on_observable::filter_on_observable_methods!();
    super::ops::filter_on_property::filter_on_property_methods!();
    super::ops::for_each_change::for_each_change_methods!();
    super::ops::for_each_item_change::for_each_item_change_methods!();
    super::ops::group_on::group_on_methods!();
    super::ops::group_on_property::group_on_property_methods!();
    super::ops::group_on_property_with_immutable_state::group_on_property_with_immutable_state_methods!();
    super::ops::group_with_immutable_state::group_with_immutable_state_methods!();
    super::ops::merge_change_sets::merge_change_sets_methods!();
    super::ops::merge_many::merge_many_methods!();
    super::ops::merge_many_change_sets::merge_many_change_sets_methods!();
    super::ops::on_item_added::on_item_added_methods!();
    super::ops::on_item_refreshed::on_item_refreshed_methods!();
    super::ops::on_item_removed::on_item_removed_methods!();
    super::ops::page::page_methods!();
    super::ops::populate_into::populate_into_methods!();
    super::ops::query_when_changed::query_when_changed_methods!();
    super::ops::ref_count::ref_count_methods!();
    super::ops::remove_index::remove_index_methods!();
    super::ops::reverse::reverse_methods!();
    super::ops::skip_initial::skip_initial_methods!();
    super::ops::sort::sort_methods!();
    super::ops::start_with_empty::start_with_empty_methods!();
    super::ops::subscribe_many::subscribe_many_methods!();
    super::ops::suppress_refresh::suppress_refresh_methods!();
    super::ops::to_collection::to_collection_methods!();
    super::ops::to_sorted_collection::to_sorted_collection_methods!();
    super::ops::top::top_methods!();
    super::ops::transform::transform_methods!();
    super::ops::transform_many::transform_many_methods!();
    super::ops::virtualise::virtualise_methods!();
    super::ops::when_any_property_changed::when_any_property_changed_methods!();
    super::ops::when_property_changed::when_property_changed_methods!();
    super::ops::when_value_changed::when_value_changed_methods!();
    super::ops::where_reasons_are::where_reasons_are_methods!();
    super::ops::where_reasons_are_not::where_reasons_are_not_methods!();

    super::ops::add_key::add_key_methods!();
    super::ops::as_aggregator::as_aggregator_methods!();
    super::ops::as_observable_list::as_observable_list_methods!();
    super::ops::collect_update_stats::collect_update_stats_methods!();
    super::ops::not_empty::not_empty_methods!();
}

impl<T: Clone + PartialEq + 'static, C: IChangeSet<T>> ObservableListEx<T> for Rc<dyn IObservable<C>> {
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        if let Some(plain) = (self as &dyn Any).downcast_ref::<Rc<dyn IObservable<ChangeSet<T>>>>() {
            return plain.clone();
        }
        self.select(|changes: C| changes.change_set().clone())
    }
}
