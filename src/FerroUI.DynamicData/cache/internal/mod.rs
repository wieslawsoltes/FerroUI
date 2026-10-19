//! The internal parts of the cache side (`DynamicData/Cache/Internal`).

mod cache;
mod cache_updater;
mod filter;
mod reader_writer;

pub(crate) use cache::Cache;
pub use cache_updater::KeySelector;
pub(crate) use cache_updater::CacheUpdater;
pub(crate) use filter::{filter_dynamic, filter_static};
pub(crate) use reader_writer::ReaderWriter;
pub(crate) mod anonymous_query;
#[allow(unused_imports)]
pub use anonymous_query::*;
pub(crate) mod auto_refresh;
#[allow(unused_imports)]
pub use auto_refresh::*;
pub(crate) mod batch_if;
#[allow(unused_imports)]
pub use batch_if::*;
pub(crate) mod bind;
#[allow(unused_imports)]
pub use bind::*;
pub(crate) mod cache_ex;
#[allow(unused_imports)]
pub use cache_ex::*;
pub(crate) mod cast;
#[allow(unused_imports)]
pub use cast::*;
pub(crate) mod change_set_cache;
#[allow(unused_imports)]
pub use change_set_cache::*;
pub(crate) mod change_set_merge_tracker;
#[allow(unused_imports)]
pub use change_set_merge_tracker::*;
pub(crate) mod defer_until_loaded;
#[allow(unused_imports)]
pub use defer_until_loaded::*;
pub(crate) mod dispose_many;
#[allow(unused_imports)]
pub use dispose_many::*;
pub(crate) mod distinct_calculator;
#[allow(unused_imports)]
pub use distinct_calculator::*;
pub(crate) mod dynamic_grouper;
#[allow(unused_imports)]
pub use dynamic_grouper::*;
pub(crate) mod edit_diff;
#[allow(unused_imports)]
pub use edit_diff::*;
pub(crate) mod edit_diff_change_set;
#[allow(unused_imports)]
pub use edit_diff_change_set::*;
pub(crate) mod edit_diff_change_set_optional;
#[allow(unused_imports)]
pub use edit_diff_change_set_optional::*;
pub(crate) mod expirable_item;
#[allow(unused_imports)]
pub use expirable_item::*;
pub(crate) mod expire_after;
#[allow(unused_imports)]
pub use expire_after::*;
pub(crate) mod expire_after_for_source;
#[allow(unused_imports)]
pub use expire_after_for_source::*;
pub(crate) mod expire_after_for_stream;
#[allow(unused_imports)]
pub use expire_after_for_stream::*;
pub(crate) mod filter_immutable;
#[allow(unused_imports)]
pub use filter_immutable::*;
pub(crate) mod filter_on_observable;
#[allow(unused_imports)]
pub use filter_on_observable::*;
pub(crate) mod filtered_index_calculator;
#[allow(unused_imports)]
pub use filtered_index_calculator::*;
pub(crate) mod finally_safe;
#[allow(unused_imports)]
pub use finally_safe::*;
pub(crate) mod for_forced;
#[allow(unused_imports)]
pub use for_forced::*;
pub(crate) mod group_on;
#[allow(unused_imports)]
pub use group_on::*;
pub(crate) mod group_on_dynamic;
#[allow(unused_imports)]
pub use group_on_dynamic::*;
pub(crate) mod group_on_immutable;
#[allow(unused_imports)]
pub use group_on_immutable::*;
pub(crate) mod group_on_observable;
#[allow(unused_imports)]
pub use group_on_observable::*;
pub(crate) mod group_on_property;
#[allow(unused_imports)]
pub use group_on_property::*;
pub(crate) mod group_on_property_with_immutable_state;
#[allow(unused_imports)]
pub use group_on_property_with_immutable_state::*;
pub(crate) mod immutable_group;
#[allow(unused_imports)]
pub use immutable_group::*;
pub(crate) mod index_calculator;
#[allow(unused_imports)]
pub use index_calculator::*;
pub(crate) mod key_comparer;
#[allow(unused_imports)]
pub use key_comparer::*;
pub(crate) mod key_value_collection;
#[allow(unused_imports)]
pub use key_value_collection::*;
pub(crate) mod key_value_comparer;
#[allow(unused_imports)]
pub use key_value_comparer::*;
pub(crate) mod managed_group;
#[allow(unused_imports)]
pub use managed_group::*;
pub(crate) mod merge_change_sets;
#[allow(unused_imports)]
pub use merge_change_sets::*;
pub(crate) mod merge_many;
#[allow(unused_imports)]
pub use merge_many::*;
pub(crate) mod merge_many_cache_change_sets;
#[allow(unused_imports)]
pub use merge_many_cache_change_sets::*;
pub(crate) mod merge_many_cache_change_sets_source_compare;
#[allow(unused_imports)]
pub use merge_many_cache_change_sets_source_compare::*;
pub(crate) mod merge_many_items;
#[allow(unused_imports)]
pub use merge_many_items::*;
pub(crate) mod merge_many_list_change_sets;
#[allow(unused_imports)]
pub use merge_many_list_change_sets::*;
pub(crate) mod observable_with_value;
#[allow(unused_imports)]
pub use observable_with_value::*;
pub(crate) mod of_type;
#[allow(unused_imports)]
pub use of_type::*;
pub(crate) mod on_being_removed;
#[allow(unused_imports)]
pub use on_being_removed::*;
pub(crate) mod on_change_action;
#[allow(unused_imports)]
pub use on_change_action::*;
pub(crate) mod page;
#[allow(unused_imports)]
pub use page::*;
pub(crate) mod query_when_changed;
#[allow(unused_imports)]
pub use query_when_changed::*;
pub(crate) mod ref_count;
#[allow(unused_imports)]
pub use ref_count::*;
pub(crate) mod remove_key_enumerator;
#[allow(unused_imports)]
pub use remove_key_enumerator::*;
pub(crate) mod size_expirer;
#[allow(unused_imports)]
pub use size_expirer::*;
pub(crate) mod size_limiter;
#[allow(unused_imports)]
pub use size_limiter::*;
pub(crate) mod sort;
#[allow(unused_imports)]
pub use sort::*;
pub(crate) mod sort_and_page;
#[allow(unused_imports)]
pub use sort_and_page::*;
pub(crate) mod sort_and_virtualize;
#[allow(unused_imports)]
pub use sort_and_virtualize::*;
pub(crate) mod sort_extensions;
#[allow(unused_imports)]
pub use sort_extensions::*;
pub(crate) mod sorted_key_value_applicator;
#[allow(unused_imports)]
pub use sorted_key_value_applicator::*;
pub(crate) mod specified_grouper;
#[allow(unused_imports)]
pub use specified_grouper::*;
pub(crate) mod status_monitor;
#[allow(unused_imports)]
pub use status_monitor::*;
pub(crate) mod subscribe_many;
#[allow(unused_imports)]
pub use subscribe_many::*;
pub(crate) mod to_collection;
#[allow(unused_imports)]
pub use to_collection::*;
pub(crate) mod to_observable_change_set;
#[allow(unused_imports)]
pub use to_observable_change_set::*;
pub(crate) mod to_observable_optional;
#[allow(unused_imports)]
pub use to_observable_optional::*;
pub(crate) mod transform;
#[allow(unused_imports)]
pub use transform::*;
pub(crate) mod transform_immutable;
#[allow(unused_imports)]
pub use transform_immutable::*;
pub(crate) mod transform_items;
#[allow(unused_imports)]
pub use transform_items::*;
pub(crate) mod transform_many;
#[allow(unused_imports)]
pub use transform_many::*;
pub(crate) mod transform_on_observable;
#[allow(unused_imports)]
pub use transform_on_observable::*;
pub(crate) mod transform_with_forced_transform;
#[allow(unused_imports)]
pub use transform_with_forced_transform::*;
pub(crate) mod transform_with_inline_update;
#[allow(unused_imports)]
pub use transform_with_inline_update::*;
pub(crate) mod tree_builder;
#[allow(unused_imports)]
pub use tree_builder::*;
pub(crate) mod true_for;
#[allow(unused_imports)]
pub use true_for::*;
pub(crate) mod uniqueness_enforcer;
#[allow(unused_imports)]
pub use uniqueness_enforcer::*;
pub(crate) mod virtualise;
#[allow(unused_imports)]
pub use virtualise::*;
// Names that more than one module defines: the ones the siblings mean.
pub(crate) use dispose_many::dispose_many;
pub(crate) use key_value_comparer::Comparer;
pub(crate) mod async_dispose_many;
#[allow(unused_imports)]
pub use async_dispose_many::*;
pub(crate) mod combine_operator;
#[allow(unused_imports)]
pub use combine_operator::*;
pub(crate) mod combiner;
#[allow(unused_imports)]
pub use combiner::*;
pub(crate) mod dynamic_combiner;
#[allow(unused_imports)]
pub use dynamic_combiner::*;
pub(crate) mod full_join;
#[allow(unused_imports)]
pub use full_join::*;
pub(crate) mod full_join_many;
#[allow(unused_imports)]
pub use full_join_many::*;
pub(crate) mod inner_join;
#[allow(unused_imports)]
pub use inner_join::*;
pub(crate) mod inner_join_many;
#[allow(unused_imports)]
pub use inner_join_many::*;
pub(crate) mod left_join;
#[allow(unused_imports)]
pub use left_join::*;
pub(crate) mod left_join_many;
#[allow(unused_imports)]
pub use left_join_many::*;
pub(crate) mod right_join;
#[allow(unused_imports)]
pub use right_join::*;
pub(crate) mod right_join_many;
#[allow(unused_imports)]
pub use right_join_many::*;
pub(crate) mod switch;
#[allow(unused_imports)]
pub use switch::*;
pub(crate) mod transform_async;
#[allow(unused_imports)]
pub use transform_async::*;
pub(crate) mod transform_many_async;
#[allow(unused_imports)]
pub use transform_many_async::*;
