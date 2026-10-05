use crate::sk_cache_base::SkCacheBase;
use skia_safe::Paint;

thread_local! {
    static SHARED: SkCacheBase<Paint> = const { SkCacheBase::new() };
}

/// Cache of paints that can be shared.
///
/// A paint is taken with [`get`](Self::get) and handed back with
/// [`return_reset`](Self::return_reset) (or [`return_item`](Self::return_item)
/// when it has already been reset).
pub struct SkPaintCache;

impl SkPaintCache {
    /// Gets a cached paint for usage.
    pub fn get() -> Paint {
        SHARED.with(|cache| cache.get())
    }

    /// Returns a paint for reuse later, without resetting it.
    pub fn return_item(paint: Paint) {
        SHARED.with(|cache| cache.return_item(paint));
    }

    /// Returns a paint and resets it for reuse later.
    ///
    /// Do not use the paint further. Do not return the same paint more than
    /// once as that will break the cache.
    pub fn return_reset(mut paint: Paint) {
        paint.reset();
        SHARED.with(|cache| cache.return_item(paint));
    }

    /// Clears all cached paints.
    pub fn clear() {
        SHARED.with(|cache| cache.clear());
    }
}
