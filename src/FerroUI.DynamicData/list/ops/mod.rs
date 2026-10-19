//! The operators of lists (`List/ObservableListEx.*.cs`): one module per upstream file, each with a
//! `*_methods!` macro of provided methods of [`ObservableListEx`](super::ObservableListEx).

pub(crate) mod add_key;
pub(crate) mod as_aggregator;
pub(crate) mod as_observable_list;
pub(crate) mod collect_update_stats;
pub(crate) mod not_empty;
