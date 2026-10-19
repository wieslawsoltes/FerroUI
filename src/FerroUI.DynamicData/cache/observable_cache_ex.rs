//! `ObservableCacheEx` (`Cache/ObservableCacheEx*.cs`): the operators of the sequences of change
//! sets of caches.
//!
//! Each upstream operator is a provided method, from the `*_methods!` macro of its module in
//! `ops/`. The trait is implemented for every sequence of change sets of any form ([`IChangeSet`]),
//! as upstream's extension methods take the interface (docs/porting/reactive.md, section 5.4).
//!
//! Naming (DEVIATIONS.md, DynamicData): an operator whose name is that of an operator of
//! `ferroui_reactive::ObservableExt`, which applies to the same sequences, takes the suffix `_cache`
//! (`and_cache`, `cast_cache`, `of_type_cache`, `switch_cache`, `count_cache`, `is_empty_cache`,
//! `sum_cache`), so that both traits can be in scope. The LINQ-named aliases of
//! `ObservableCacheAlias` (`Select`, `Where`, `SelectMany`) are not ported for the same reason.

use super::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::any::Any;
use std::hash::Hash;
use std::rc::Rc;

/// The operators of caches (`ObservableCacheEx`).
pub trait ObservableCacheEx<T: Clone + 'static, K: Eq + Hash + Clone + 'static> {
    /// The sequence as plain change sets (the upcast of a sorted, paged or contextual change set to
    /// `IChangeSet<TObject, TKey>`).
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>;

    super::ops::adapt::adapt_methods!();
    super::ops::auto_refresh::auto_refresh_methods!();
    super::ops::auto_refresh_on_observable::auto_refresh_on_observable_methods!();
    super::ops::batch::batch_methods!();
    super::ops::batch_if::batch_if_methods!();
    super::ops::bind::bind_methods!();
    super::ops::bind_to_observable_list::bind_to_observable_list_methods!();
    super::ops::buffer_initial::buffer_initial_methods!();
    super::ops::cast::cast_methods!();
    super::ops::change_key::change_key_methods!();
    super::ops::clone::clone_methods!();
    super::ops::convert::convert_methods!();
    super::ops::defer_until_loaded::defer_until_loaded_methods!();
    super::ops::dispose_many::dispose_many_methods!();
    super::ops::distinct_values::distinct_values_methods!();
    super::ops::ensure_unique_keys::ensure_unique_keys_methods!();
    super::ops::expire_after::expire_after_methods!();
    super::ops::filter_immutable::filter_immutable_methods!();
    super::ops::filter_on_observable::filter_on_observable_methods!();
    super::ops::flatten::flatten_methods!();
    super::ops::for_each_change::for_each_change_methods!();
    super::ops::group::group_methods!();
    super::ops::group_on_observable::group_on_observable_methods!();
    super::ops::group_on_property::group_on_property_methods!();
    super::ops::group_on_property_with_immutable_state::group_on_property_with_immutable_state_methods!();
    super::ops::group_with_immutable_state::group_with_immutable_state_methods!();
    super::ops::ignore_same_reference_update::ignore_same_reference_update_methods!();
    super::ops::ignore_update_when::ignore_update_when_methods!();
    super::ops::include_update_when::include_update_when_methods!();
    super::ops::invoke_evaluate::invoke_evaluate_methods!();
    super::ops::limit_size_to::limit_size_to_methods!();
    super::ops::merge_change_sets::merge_change_sets_methods!();
    super::ops::merge_many::merge_many_methods!();
    super::ops::merge_many_change_sets::merge_many_change_sets_methods!();
    super::ops::merge_many_items::merge_many_items_methods!();
    super::ops::of_type::of_type_methods!();
    super::ops::on_item_added::on_item_added_methods!();
    super::ops::on_item_refreshed::on_item_refreshed_methods!();
    super::ops::on_item_removed::on_item_removed_methods!();
    super::ops::on_item_updated::on_item_updated_methods!();
    super::ops::populate_into::populate_into_methods!();
    super::ops::query_when_changed::query_when_changed_methods!();
    super::ops::ref_count::ref_count_methods!();
    super::ops::remove_key::remove_key_methods!();
    super::ops::skip_initial::skip_initial_methods!();
    super::ops::sort::sort_methods!();
    super::ops::sort_and_bind::sort_and_bind_methods!();
    super::ops::sort_by::sort_by_methods!();
    super::ops::start_with_empty::start_with_empty_methods!();
    super::ops::start_with_item::start_with_item_methods!();
    super::ops::subscribe_many::subscribe_many_methods!();
    super::ops::suppress_refresh::suppress_refresh_methods!();
    super::ops::to_collection::to_collection_methods!();
    super::ops::to_observable_optional::to_observable_optional_methods!();
    super::ops::to_sorted_collection::to_sorted_collection_methods!();
    super::ops::transform::transform_methods!();
    super::ops::transform_immutable::transform_immutable_methods!();
    super::ops::transform_many::transform_many_methods!();
    super::ops::transform_on_observable::transform_on_observable_methods!();
    super::ops::transform_safe::transform_safe_methods!();
    super::ops::transform_to_tree::transform_to_tree_methods!();
    super::ops::transform_with_inline_update::transform_with_inline_update_methods!();
    super::ops::true_for_all::true_for_all_methods!();
    super::ops::true_for_any::true_for_any_methods!();
    super::ops::virtualise_and_page::virtualise_and_page_methods!();
    super::ops::watch::watch_methods!();
    super::ops::watch_value::watch_value_methods!();
    super::ops::when_any_property_changed::when_any_property_changed_methods!();
    super::ops::when_property_changed::when_property_changed_methods!();
    super::ops::when_value_changed::when_value_changed_methods!();
    super::ops::where_reasons_are::where_reasons_are_methods!();
    super::ops::where_reasons_are_not::where_reasons_are_not_methods!();

    super::ops::as_aggregator::as_aggregator_methods!();
    super::ops::as_observable_cache::as_observable_cache_methods!();
    super::ops::collect_update_stats::collect_update_stats_methods!();
    super::ops::filter::filter_methods!();
    super::ops::not_empty::not_empty_methods!();
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: IChangeSet<T, K>> ObservableCacheEx<T, K> for Rc<dyn IObservable<C>> {
    fn change_sets(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        if let Some(plain) = (self as &dyn Any).downcast_ref::<Rc<dyn IObservable<ChangeSet<T, K>>>>() {
            return plain.clone();
        }
        self.select(|changes: C| changes.change_set().clone())
    }
}
