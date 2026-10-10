//! How an object the crate exports on a connection reaches the object of
//! the UI thread it stands for (no upstream file: the D-Bus library of the
//! reference calls a handler on the synchronization context of the thread
//! that created the connection).
//!
//! The D-Bus library calls the methods of an exported object on the
//! thread of the connection, and such an object has to be `Send + Sync`.
//! The state it answers from (a menu, a tray icon) belongs to the UI
//! thread. So the exported object holds a [`UiThreadHandle`]: a number and
//! the dispatcher of the UI thread. A call posts a job to that dispatcher,
//! the job finds the object of the UI thread by the number and computes
//! the answer there, and the method of the exported object awaits it.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

thread_local! {
    /// The objects of this thread that exported objects stand for.
    static OBJECTS: RefCell<HashMap<u64, Rc<dyn Any>>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The answer of one call, filled in on the UI thread.
struct Slot<R> {
    /// `Some(None)` when the job ran and the object was gone, or when the
    /// job was dropped without running.
    value: Option<Option<R>>,
    waker: Option<Waker>,
}

struct Sender<R>(Arc<Mutex<Slot<R>>>);

impl<R> Sender<R> {
    fn complete(&self, value: Option<R>) {
        let waker = {
            let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if slot.value.is_some() {
                return;
            }
            slot.value = Some(value);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<R> Drop for Sender<R> {
    fn drop(&mut self) {
        // A job that never ran (the dispatcher shut down) answers "gone".
        self.complete(None);
    }
}

/// The future of [`UiThreadHandle::call`].
pub struct UiThreadCall<R>(Arc<Mutex<Slot<R>>>);

impl<R> Future for UiThreadCall<R> {
    type Output = Option<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<R>> {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// A handle any thread can hold to an object of the UI thread.
pub struct UiThreadHandle<T: 'static> {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for UiThreadHandle<T> {
    fn clone(&self) -> Self {
        Self { id: self.id, dispatcher: self.dispatcher.clone(), marker: PhantomData }
    }
}

impl<T: 'static> UiThreadHandle<T> {
    /// Makes `object` reachable. The object stays alive until
    /// [`unregister`](Self::unregister).
    ///
    /// # Panics
    /// Panics when called from a thread other than the UI thread.
    pub fn register(object: Rc<T>) -> Self {
        let dispatcher = Dispatcher::ui_thread();
        dispatcher.verify_access();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        OBJECTS.with(|objects| objects.borrow_mut().insert(id, object as Rc<dyn Any>));
        Self { id, dispatcher, marker: PhantomData }
    }

    /// Forgets the object: calls that have not run yet, and later ones,
    /// answer `None`. Must be called on the UI thread; on another thread
    /// it does nothing.
    pub fn unregister(&self) {
        if self.dispatcher.check_access() {
            OBJECTS.with(|objects| objects.borrow_mut().remove(&self.id));
        }
    }

    /// Runs `call` with the object on the UI thread, as a job of its
    /// dispatcher at input priority (the priority the reference creates
    /// its connections with), and resolves to what it returned; `None`
    /// when the object is not registered any more.
    pub fn call<R: Send + 'static>(&self, call: impl FnOnce(&Rc<T>) -> R + Send + 'static) -> UiThreadCall<R> {
        let slot = Arc::new(Mutex::new(Slot { value: None, waker: None }));
        let sender = Sender(slot.clone());
        let id = self.id;
        self.dispatcher.post(
            move || {
                let object = OBJECTS.with(|objects| objects.borrow().get(&id).cloned());
                let object = object.and_then(|object| object.downcast::<T>().ok());
                sender.complete(object.map(|object| call(&object)));
            },
            DispatcherPriority::INPUT,
        );
        UiThreadCall(slot)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;
    use crate::test_support::{pump_until, scope};
    use std::cell::Cell;

    fn wait<R: Send + 'static>(call: UiThreadCall<R>) -> Option<R> {
        // Another thread awaits the call, as a connection does.
        let waiter = std::thread::spawn(move || zbus::block_on(call));
        pump_until(|| waiter.is_finished());
        waiter.join().unwrap()
    }

    #[test]
    fn a_call_from_another_thread_runs_on_the_ui_thread() {
        let _scope = scope();
        let object = Rc::new(Cell::new(5));
        let handle = UiThreadHandle::register(object.clone());

        let ui_thread = std::thread::current().id();
        let call = handle.call(move |object| {
            assert_eq!(std::thread::current().id(), ui_thread);
            object.set(object.get() + 1);
            object.get()
        });
        // Nothing runs before the dispatcher does.
        assert_eq!(object.get(), 5);
        assert_eq!(wait(call), Some(6));

        handle.unregister();
        assert_eq!(wait(handle.call(|object| object.get())), None);
        assert_eq!(Rc::strong_count(&object), 1);
    }

    #[test]
    fn a_call_that_was_posted_before_the_object_went_answers_gone() {
        let _scope = scope();
        let handle = UiThreadHandle::register(Rc::new(1));
        let call = handle.call(|object| **object);
        handle.unregister();
        assert_eq!(wait(call), None);
    }
}
