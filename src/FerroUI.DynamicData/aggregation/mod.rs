//! The aggregations of DynamicData (`DynamicData/Aggregation`): change sets shaped for continuous
//! aggregations (`ForAggregation`, [`IAggregateChangeSet`], [`AggregateItem`]) and the running
//! aggregates of caches and lists: the average (`avg`), the count and whether there are items
//! (`count_cache`, `is_empty_cache`, `is_not_empty`, ...), the maximum and minimum, the sample
//! standard deviation (`std_dev`) and the sum (`sum_cache`, `sum_list`).
//!
//! The operators are methods of [`AggregationCacheEx`], [`AggregationListEx`],
//! [`AggregateChangeSetEx`] and [`InvalidateWhenEx`] (upstream's namespace is imported on its own,
//! as these traits are).

mod aggregate_enumerator;
mod aggregate_item;
mod aggregate_number;
mod aggregate_type;
mod aggregation_ex;
mod avg;
mod avg_ex;
mod count_ex;
mod i_aggregate_change_set;
mod max_ex;
mod std_dev;
mod std_dev_ex;
mod sum_ex;

pub(crate) use aggregate_enumerator::{cache_aggregate, list_aggregate};
pub use aggregate_item::AggregateItem;
pub use aggregate_number::{AggregateNumber, AggregateValue};
pub use aggregate_type::AggregateType;
pub(crate) use aggregation_ex::accumulate;
pub use aggregation_ex::{AggregateChangeSetEx, AggregationCacheEx, AggregationListEx, InvalidateWhenEx};
pub(crate) use avg::Avg;
pub use i_aggregate_change_set::{AggregateChangeSets, IAggregateChangeSet};
pub(crate) use max_ex::MaxOrMin;
pub(crate) use std_dev::StdDev;
pub use std_dev_ex::StdDevValue;
