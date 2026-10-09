//! How a handler of a connection reaches an object of the UI thread.
//!
//! The handlers of a connection run on its reader thread (the threading
//! contract of the remote protocol), where the original captures `this` in
//! the delegate it posts to the dispatcher. An object of the UI thread
//! cannot be captured by a closure that crosses threads, so the object is
//! registered on its thread under a number, and what crosses is the number
//! and the dispatcher of that thread: plain data. The posted action looks
//! the object up again on the UI thread. (Not from upstream.)

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

struct Entry {
    value: Box<dyn Any>,
    is_alive: Box<dyn Fn() -> bool>,
}

thread_local! {
    static OBJECTS: RefCell<HashMap<u64, Entry>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The name of a value registered on the UI thread, with the dispatcher of
/// that thread. It is shared between threads; the value (a weak reference
/// to an object of the UI thread) never leaves its thread.
pub struct UiThreadHandle<T> {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    marker: PhantomData<fn(T)>,
}

impl<T> Clone for UiThreadHandle<T> {
    fn clone(&self) -> Self {
        Self { id: self.id, dispatcher: self.dispatcher.clone(), marker: PhantomData }
    }
}

impl<T: Clone + 'static> UiThreadHandle<T> {
    /// Registers `value` on the calling thread, which is the UI thread: its
    /// dispatcher (`Dispatcher.UIThread` of the original, read here because
    /// on the reader thread of a connection the name would mean another
    /// dispatcher in a unit test) is the one actions are posted to.
    /// `is_alive` tells whether the value still refers to something; the
    /// registrations whose values do not are dropped by a later
    /// registration.
    pub fn register(value: T, is_alive: impl Fn(&T) -> bool + 'static) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        OBJECTS.with(|objects| {
            let mut objects = objects.borrow_mut();
            objects.retain(|_, entry| (entry.is_alive)());
            let probe = value.clone();
            objects.insert(id, Entry { value: Box::new(value), is_alive: Box::new(move || is_alive(&probe)) });
        });
        Self { id, dispatcher: Dispatcher::ui_thread(), marker: PhantomData }
    }

    /// The dispatcher of the thread the value is registered on.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    /// `Dispatcher.UIThread.Post(action, priority)` with the registered
    /// value: may be called from any thread. The action is not run when the
    /// registration has been removed.
    pub fn post(&self, action: impl FnOnce(T) + Send + 'static, priority: DispatcherPriority) {
        let id = self.id;
        self.dispatcher.post(
            move || {
                if let Some(value) = Self::resolve(id) {
                    action(value);
                }
            },
            priority,
        );
    }

    /// Removes the registration. Called on the thread that registered.
    pub fn unregister(&self) {
        let id = self.id;
        let entry = OBJECTS.try_with(|objects| objects.borrow_mut().remove(&id)).ok().flatten();
        drop(entry);
    }

    fn resolve(id: u64) -> Option<T> {
        OBJECTS.with(|objects| objects.borrow().get(&id).and_then(|entry| entry.value.downcast_ref::<T>().cloned()))
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::cell::Cell;
    use std::rc::{Rc, Weak};

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn the_handle_is_shared_between_threads() {
        assert_send_sync::<UiThreadHandle<Weak<Cell<i32>>>>();
    }

    #[test]
    fn an_action_posted_from_another_thread_runs_with_the_value_on_the_registering_thread() {
        let _scope = Dispatcher::unit_test_scope();
        let value = Rc::new(Cell::new(0));
        let handle = UiThreadHandle::register(Rc::downgrade(&value), |weak| weak.strong_count() > 0);

        let posted = handle.clone();
        std::thread::spawn(move || {
            posted.post(
                |value| {
                    if let Some(value) = value.upgrade() {
                        value.set(value.get() + 1);
                    }
                },
                DispatcherPriority::DEFAULT,
            )
        })
        .join()
        .unwrap();
        assert_eq!(0, value.get());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, value.get());

        // After the registration is removed the action is not run.
        handle.unregister();
        handle.post(|_| panic!("the registration was removed"), DispatcherPriority::DEFAULT);
        Dispatcher::ui_thread().run_jobs(None);
    }

    #[test]
    fn a_registration_whose_value_is_gone_is_dropped_by_the_next_one() {
        let _scope = Dispatcher::unit_test_scope();
        let first = Rc::new(Cell::new(0));
        let first_handle = UiThreadHandle::register(Rc::downgrade(&first), |weak| weak.strong_count() > 0);
        drop(first);
        let second = Rc::new(Cell::new(0));
        let _second_handle = UiThreadHandle::register(Rc::downgrade(&second), |weak| weak.strong_count() > 0);
        assert!(UiThreadHandle::<Weak<Cell<i32>>>::resolve(first_handle.id).is_none());
    }
}
