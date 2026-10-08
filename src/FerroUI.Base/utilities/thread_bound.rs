use std::mem::ManuallyDrop;
use std::thread::{self, ThreadId};

/// A value that stays on the thread that created it, inside an object that
/// is shared between threads.
///
/// A render resource is shared between the UI thread and the render thread,
/// yet some of them keep a part that only one of the threads uses (the
/// render target a render target bitmap draws itself with, for example).
/// The part is reached through [`get`](Self::get), which panics on any other
/// thread, so the object as a whole can be `Send + Sync`.
///
/// When the object is dropped on another thread the value is not dropped but
/// leaked: its destructor may only run on its own thread. Owners release
/// what the value holds from their `dispose`, on the right thread.
pub struct ThreadBound<T> {
    value: ManuallyDrop<T>,
    thread: ThreadId,
}

// SAFETY: the value is only reached, and only dropped, on the thread that
// created it; every other thread sees an opaque box.
unsafe impl<T> Send for ThreadBound<T> {}
unsafe impl<T> Sync for ThreadBound<T> {}

impl<T> ThreadBound<T> {
    /// Binds `value` to the current thread.
    pub fn new(value: T) -> Self {
        Self { value: ManuallyDrop::new(value), thread: thread::current().id() }
    }

    /// Whether the current thread is the one the value is bound to.
    pub fn is_on_thread(&self) -> bool {
        thread::current().id() == self.thread
    }

    /// The value.
    ///
    /// # Panics
    ///
    /// On a thread other than the one the value was created on.
    pub fn get(&self) -> &T {
        assert!(self.is_on_thread(), "the value is bound to the thread that created it");
        &self.value
    }
}

impl<T> Drop for ThreadBound<T> {
    fn drop(&mut self) {
        if self.is_on_thread() {
            // SAFETY: the value is not used after this, and this is its thread.
            unsafe { ManuallyDrop::drop(&mut self.value) };
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn the_value_is_reached_and_dropped_on_its_thread() {
        let value = Rc::new(5);
        let bound = ThreadBound::new(value.clone());

        assert_eq!(5, **bound.get());
        drop(bound);
        assert_eq!(1, Rc::strong_count(&value));
    }

    #[test]
    fn another_thread_cannot_reach_the_value_and_leaks_it() {
        let value = Rc::new(5);
        let bound = Arc::new(ThreadBound::new(value.clone()));

        let moved = bound.clone();
        let reached = thread::spawn(move || std::panic::catch_unwind(|| moved.is_on_thread()).unwrap_or(true))
            .join()
            .unwrap();
        assert!(!reached);

        let moved = bound;
        thread::spawn(move || drop(moved)).join().unwrap();
        // Not dropped: the count of the handle is untouched by the other thread.
        assert_eq!(2, Rc::strong_count(&value));
    }
}
