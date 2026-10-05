use std::cell::RefCell;

/// Cache base for Skia objects.
///
/// A cache is a per-thread pool: [`get`](Self::get) takes an item out of it
/// (creating one when it is empty) and [`return_item`](Self::return_item)
/// puts it back.
pub struct SkCacheBase<TCachedItem: Default> {
    cache: RefCell<Vec<TCachedItem>>,
}

impl<TCachedItem: Default> SkCacheBase<TCachedItem> {
    /// Creates an empty cache.
    pub const fn new() -> Self {
        Self { cache: RefCell::new(Vec::new()) }
    }

    /// Gets a cached item for usage.
    ///
    /// If there is a valid item in the cache it is taken, otherwise a new
    /// item is created.
    pub fn get(&self) -> TCachedItem {
        self.cache.borrow_mut().pop().unwrap_or_default()
    }

    /// Returns the item for reuse later.
    ///
    /// Do not use the item further. Do not return the same item more than
    /// once as that will break the cache.
    pub fn return_item(&self, item: TCachedItem) {
        self.cache.borrow_mut().push(item);
    }

    /// Clears and disposes all cached items.
    ///
    /// Typically called when the application is shutting down or going to
    /// the background.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl<TCachedItem: Default> Default for SkCacheBase<TCachedItem> {
    fn default() -> Self {
        Self::new()
    }
}
