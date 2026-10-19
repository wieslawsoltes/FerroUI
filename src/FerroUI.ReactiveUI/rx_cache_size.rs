//! `RxCacheSize` (`ReactiveUI.Shared/RxCacheSize.cs`).

use std::cell::Cell;

/// The limit of the small caches on mobile targets (`DefaultSmallCacheLimit`).
#[cfg(any(target_os = "android", target_os = "ios"))]
const DEFAULT_SMALL_CACHE_LIMIT: usize = 32;
/// The limit of the big caches on mobile targets (`DefaultBigCacheLimit`).
#[cfg(any(target_os = "android", target_os = "ios"))]
const DEFAULT_BIG_CACHE_LIMIT: usize = 64;
/// The limit of the small caches (`DefaultSmallCacheLimit`).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const DEFAULT_SMALL_CACHE_LIMIT: usize = 64;
/// The limit of the big caches (`DefaultBigCacheLimit`).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const DEFAULT_BIG_CACHE_LIMIT: usize = 256;

ferroui_system::ferro_thread_local! {
    /// The limits, once read or initialized (`_limits`).
    static LIMITS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

/// The sizes of the caches of ReactiveUI (`RxCacheSize`): the memoizing caches of view location
/// and of the observation of properties.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the limits are per thread, as the caches that read
/// them are.
pub struct RxCacheSize;

impl RxCacheSize {
    /// The limit of the small caches (`SmallCacheLimit`).
    pub fn small_cache_limit() -> usize {
        Self::limits().0
    }

    /// The limit of the big caches (`BigCacheLimit`).
    pub fn big_cache_limit() -> usize {
        Self::limits().1
    }

    /// Sets the limits unless they were set or read before (`Initialize`); the builder of the
    /// application calls it.
    pub fn initialize(small_cache_limit: usize, big_cache_limit: usize) {
        LIMITS.with(|limits| {
            if limits.get().is_none() {
                limits.set(Some((small_cache_limit, big_cache_limit)));
            }
        });
    }

    /// Forgets the limits, so that the next read or initialization sets them again
    /// (`ResetForTesting`); for tests.
    #[doc(hidden)]
    pub fn reset_for_testing() {
        LIMITS.with(|limits| limits.set(None));
    }

    fn limits() -> (usize, usize) {
        LIMITS.with(|limits| match limits.get() {
            Some(current) => current,
            None => {
                let defaults = (DEFAULT_SMALL_CACHE_LIMIT, DEFAULT_BIG_CACHE_LIMIT);
                limits.set(Some(defaults));
                defaults
            }
        })
    }
}
