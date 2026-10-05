use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Creates explicitly counted references; see [`RefCounted`].
pub struct RefCountable;

impl RefCountable {
    /// Creates the first counted reference to `item`. `release` runs once,
    /// when the last reference is disposed (or dropped).
    pub fn create<T: ?Sized + 'static>(item: Rc<T>, release: impl FnOnce() + 'static) -> RefCounted<T> {
        RefCounted { item: RefCell::new(Some(item)), counter: Rc::new(RefCounter::new(Box::new(release))) }
    }
}

struct RefCounter {
    release: Cell<Option<Box<dyn FnOnce()>>>,
    refs: Cell<i32>,
}

impl RefCounter {
    fn new(release: Box<dyn FnOnce()>) -> Self {
        Self { release: Cell::new(Some(release)), refs: Cell::new(1) }
    }

    fn try_add_ref(&self) -> bool {
        let old = self.refs.get();
        if old == 0 {
            return false;
        }
        self.refs.set(old + 1);
        true
    }

    fn release(&self) {
        let old = self.refs.get();
        self.refs.set(old - 1);
        if old == 1 {
            if let Some(release) = self.release.take() {
                release();
            }
        }
    }

    fn ref_count(&self) -> i32 {
        self.refs.get()
    }
}

/// An explicitly counted reference to a shared resource.
///
/// Every reference is disposed on its own; the resource is released when the
/// last one is disposed. A reference that is dropped without having been
/// disposed releases its count as well.
pub struct RefCounted<T: ?Sized + 'static> {
    item: RefCell<Option<Rc<T>>>,
    counter: Rc<RefCounter>,
}

impl<T: ?Sized + 'static> RefCounted<T> {
    /// Releases this reference.
    pub fn dispose(&self) {
        let item = self.item.borrow_mut().take();
        if item.is_some() {
            self.counter.release();
        }
    }

    /// The referenced resource. Panics if this reference has been disposed.
    pub fn item(&self) -> Rc<T> {
        self.item.borrow().clone().unwrap_or_else(|| Self::disposed())
    }

    /// Creates another reference to the resource. Panics if this reference
    /// has been disposed.
    pub fn clone_ref(&self) -> RefCounted<T> {
        self.clone_as(|item| item)
    }

    /// Creates another reference to the resource, viewed as another type.
    /// Panics if this reference has been disposed.
    pub fn clone_as<TResult: ?Sized + 'static>(&self, cast: impl FnOnce(Rc<T>) -> Rc<TResult>) -> RefCounted<TResult> {
        // Snapshot the item so it does not matter if the cast disposes this
        // reference in the meantime.
        let item = self.item.borrow().clone();
        let Some(item) = item else { Self::disposed() };

        // Try to add a reference to the counter; if it fails, the item is
        // released.
        if !self.counter.try_add_ref() {
            Self::disposed();
        }

        RefCounted { item: RefCell::new(Some(cast(item))), counter: self.counter.clone() }
    }

    /// Whether this reference has not been disposed.
    pub fn is_alive(&self) -> bool {
        self.item.borrow().is_some()
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
        let released = Rc::new(Cell::new(0));
        let r = released.clone();
        let first = RefCountable::create(Rc::new(5), move || r.set(r.get() + 1));
        assert_eq!(1, first.ref_count());

        let second = first.clone_ref();
        assert_eq!(2, first.ref_count());
        assert_eq!(5, *second.item());

        first.dispose();
        first.dispose();
        assert!(!first.is_alive());
        assert!(second.is_alive());
        assert_eq!(1, second.ref_count());
        assert_eq!(0, released.get());

        let third = second.clone_as(|item| item as Rc<dyn std::any::Any>);
        second.dispose();
        assert_eq!(0, released.get());
        drop(third);
        assert_eq!(1, released.get());
    }

    #[test]
    #[should_panic(expected = "disposed")]
    fn item_of_a_disposed_reference_panics() {
        let reference = RefCountable::create(Rc::new(1), || {});
        reference.dispose();
        reference.item();
    }

    #[test]
    #[should_panic(expected = "disposed")]
    fn cloning_a_disposed_reference_panics() {
        let reference = RefCountable::create(Rc::new(1), || {});
        reference.dispose();
        reference.clone_ref();
    }
}
