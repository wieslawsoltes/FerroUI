use std::cell::Cell;
use std::ffi::c_void;

#[link(name = "objc")]
extern "C" {
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

/// An Objective-C autorelease pool that is drained when it is disposed (or
/// dropped).
///
/// Pools nest: a pool must be disposed on the thread that created it, after
/// every pool created later on that thread.
pub struct AutoReleasePool {
    pool: Cell<*mut c_void>,
}

impl AutoReleasePool {
    /// Pushes a new autorelease pool.
    pub fn new() -> Self {
        // SAFETY: pushing an autorelease pool has no preconditions.
        Self { pool: Cell::new(unsafe { objc_autoreleasePoolPush() }) }
    }

    /// Pops the pool, releasing the objects autoreleased since it was
    /// pushed. Does nothing when called again.
    pub fn dispose(&self) {
        let pool = self.pool.replace(std::ptr::null_mut());
        if !pool.is_null() {
            // SAFETY: `pool` came from `objc_autoreleasePoolPush` and is
            // popped exactly once, because the cell is cleared first.
            unsafe { objc_autoreleasePoolPop(pool) };
        }
    }
}

impl Default for AutoReleasePool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AutoReleasePool {
    fn drop(&mut self) {
        self.dispose();
    }
}
