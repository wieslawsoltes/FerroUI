use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

/// Creates explicitly counted references; see [`RefCounted`].
pub struct RefCountable;

impl RefCountable {
    /// Creates the first counted reference to `item`. `release` runs once,
    /// when the last reference is disposed (or dropped).
    pub fn create<T: ?Sized + 'static>(item: Arc<T>, release: impl FnOnce() + Send + 'static) -> RefCounted<T> {
        RefCounted { item: Mutex::new(Some(item)), counter: Arc::new(RefCounter::new(Box::new(release))) }
    }
}

struct RefCounter {
    release: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    refs: AtomicI32,
}

impl RefCounter {
    fn new(release: Box<dyn FnOnce() + Send>) -> Self {
        Self { release: Mutex::new(Some(release)), refs: AtomicI32::new(1) }
    }

    fn try_add_ref(&self) -> bool {
        let mut old = self.refs.load(Ordering::SeqCst);
        loop {
            if old == 0 {
                return false;
            }
            match self.refs.compare_exchange(old, old + 1, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return true,
                Err(current) => old = current,
            }
        }
    }

    fn release(&self) {
        let old = self.refs.fetch_sub(1, Ordering::SeqCst);
        if old == 1 {
            let release = self.release.lock().unwrap().take();
            if let Some(release) = release {
                release();
            }
        }
    }

    fn ref_count(&self) -> i32 {
        self.refs.load(Ordering::SeqCst)
    }
}

/// An explicitly counted reference to a shared resource.
///
/// Every reference is disposed on its own; the resource is released when the
/// last one is disposed. A reference that is dropped without having been
/// disposed releases its count as well.
///
/// References are shared between the UI thread and the render thread: the
/// count is atomic, as in the original.
pub struct RefCounted<T: ?Sized + 'static> {
    item: Mutex<Option<Arc<T>>>,
    counter: Arc<RefCounter>,
}

impl<T: ?Sized + 'static> RefCounted<T> {
    /// Releases this reference.
    pub fn dispose(&self) {
        let item = self.item.lock().unwrap().take();
        if item.is_some() {
            self.counter.release();
        }
    }

    /// The referenced resource. Panics if this reference has been disposed.
    pub fn item(&self) -> Arc<T> {
        // The lock is released before a disposed reference panics.
        let item = self.item.lock().unwrap().clone();
        item.unwrap_or_else(|| Self::disposed())
    }

    /// Creates another reference to the resource. Panics if this reference
    /// has been disposed.
    pub fn clone_ref(&self) -> RefCounted<T> {
        self.clone_as(|item| item)
    }

    /// Creates another reference to the resource, viewed as another type.
    /// Panics if this reference has been disposed.
    pub fn clone_as<TResult: ?Sized + 'static>(&self, cast: impl FnOnce(Arc<T>) -> Arc<TResult>) -> RefCounted<TResult> {
        // Snapshot the item so it does not matter if the cast disposes this
        // reference in the meantime.
        let item = self.item.lock().unwrap().clone();
        let Some(item) = item else { Self::disposed() };

        // Try to add a reference to the counter; if it fails, the item is
        // released.
        if !self.counter.try_add_ref() {
            Self::disposed();
        }

        RefCounted { item: Mutex::new(Some(cast(item))), counter: self.counter.clone() }
    }

    /// Whether this reference has not been disposed.
    pub fn is_alive(&self) -> bool {
        self.item.lock().unwrap().is_some()
    }

    /// The number of live references to the resource. Panics if this
    /// reference has been disposed.
    pub fn ref_count(&self) -> i32 {
        if !self.is_alive() {
            Self::disposed();
        }
        self.counter.ref_count()
    }

    fn disposed() -> ! {
        panic!("Cannot access a disposed object: RefCounted<{}>", std::any::type_name::<T>())
    }
}

impl<T: ?Sized + 'static> Drop for RefCounted<T> {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn releases_when_the_last_reference_is_disposed() {
        let released = Arc::new(AtomicI32::new(0));
        let r = released.clone();
        let first = RefCountable::create(Arc::new(5), move || {
            r.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(1, first.ref_count());

        let second = first.clone_ref();
        assert_eq!(2, first.ref_count());
        assert_eq!(5, *second.item());

        first.dispose();
        first.dispose();
        assert!(!first.is_alive());
        assert!(second.is_alive());
        assert_eq!(1, second.ref_count());
        assert_eq!(0, released.load(Ordering::SeqCst));

        let third = second.clone_as(|item| item as Arc<dyn std::any::Any + Send + Sync>);
        second.dispose();
        assert_eq!(0, released.load(Ordering::SeqCst));
        drop(third);
        assert_eq!(1, released.load(Ordering::SeqCst));
    }

    #[test]
    #[should_panic(expected = "disposed")]
    fn item_of_a_disposed_reference_panics() {
        let reference = RefCountable::create(Arc::new(1), || {});
        reference.dispose();
        reference.item();
    }

    #[test]
    #[should_panic(expected = "disposed")]
    fn cloning_a_disposed_reference_panics() {
        let reference = RefCountable::create(Arc::new(1), || {});
        reference.dispose();
        reference.clone_ref();
    }
}
