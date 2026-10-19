//! The operators of caches (`Cache/ObservableCacheEx.*.cs`): one module per upstream file, each
//! with a `*_methods!` macro of provided methods of [`ObservableCacheEx`](super::ObservableCacheEx).

pub(crate) mod filter;
pub(crate) mod not_empty;
