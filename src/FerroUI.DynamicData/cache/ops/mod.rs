//! The operators of caches (`Cache/ObservableCacheEx.*.cs`): one module per upstream file, each
//! with a `*_methods!` macro of provided methods of [`ObservableCacheEx`](super::ObservableCacheEx).

pub(crate) mod as_aggregator;
pub(crate) mod as_observable_cache;
pub(crate) mod collect_update_stats;
pub(crate) mod filter;
pub(crate) mod not_empty;
