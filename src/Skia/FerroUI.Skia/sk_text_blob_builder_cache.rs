use crate::sk_cache_base::SkCacheBase;
use skia_safe::TextBlobBuilder;

thread_local! {
    static SHARED: SkCacheBase<Builder> = const { SkCacheBase::new() };
}

/// A text blob builder that can be default-constructed for the cache.
struct Builder(TextBlobBuilder);

impl Default for Builder {
    fn default() -> Self {
        Self(TextBlobBuilder::new())
    }
}

/// Cache of text blob builders that can be shared.
pub struct SkTextBlobBuilderCache;

impl SkTextBlobBuilderCache {
    /// Gets a cached builder for usage.
    pub fn get() -> TextBlobBuilder {
        SHARED.with(|cache| cache.get()).0
    }

    /// Returns a builder for reuse later.
    pub fn return_item(builder: TextBlobBuilder) {
        SHARED.with(|cache| cache.return_item(Builder(builder)));
    }

    /// Clears all cached builders.
    pub fn clear() {
        SHARED.with(|cache| cache.clear());
    }
}
