//! `RecordingObserverBase<T>` (`Utilities/RecordingObserverBase.cs`).

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::subjects::AsyncSubject;
use ferroui_reactive::Notification;
use ferroui_reactive_testing::{duration_to_ticks, Recorded};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::SystemTime;

/// The part of the recording observers that records every notification with the time of its
/// scheduler, the error and the end (`RecordingObserverBase<T>`).
///
/// Upstream's abstract class is a part that each observer holds: the observer records the
/// notification here, then handles the value itself (`OnNext`). The observers have no safeguards,
/// so that tests can see what a sequence does wrong.
pub struct RecordingObserverBase<T> {
    notifications: RefCell<Vec<Recorded<Notification<T>>>>,
    scheduler: Rc<dyn IScheduler>,
    error: RefCell<Option<ObservableError>>,
    has_completed: Cell<bool>,
    when_finalized: Rc<AsyncSubject<()>>,
}

impl<T: Clone + 'static> RecordingObserverBase<T> {
    /// A part that stamps the notifications with the time of `scheduler`.
    pub fn new(scheduler: Rc<dyn IScheduler>) -> Self {
        Self { notifications: RefCell::new(Vec::new()), scheduler, error: RefCell::new(None), has_completed: Cell::new(false), when_finalized: AsyncSubject::new() }
    }

    /// The first error received, when the sequence had not ended before it (`Error`).
    pub fn error(&self) -> Option<ObservableError> {
        self.error.borrow().clone()
    }

    /// Whether the sequence has completed (`HasCompleted`).
    pub fn has_completed(&self) -> bool {
        self.has_completed.get()
    }

    /// Whether the sequence has completed or failed (`HasFinalized`).
    pub fn has_finalized(&self) -> bool {
        self.has_completed.get() || self.error.borrow().is_some()
    }

    /// Every notification received, with the time of the scheduler in ticks (`Notifications`).
    pub fn notifications(&self) -> Vec<Recorded<Notification<T>>> {
        self.notifications.borrow().clone()
    }

    /// One value and the end when the sequence completes or fails (`WhenFinalized`, a task
    /// upstream).
    pub fn when_finalized(&self) -> Rc<dyn IObservable<()>> {
        self.when_finalized.as_observable()
    }

    fn now_ticks(&self) -> i64 {
        self.scheduler.now().duration_since(SystemTime::UNIX_EPOCH).map(duration_to_ticks).unwrap_or(0)
    }

    fn finalize(&self) {
        if !self.when_finalized.is_completed() {
            self.when_finalized.on_next(());
            self.when_finalized.on_completed();
        }
    }

    /// Records a value (`IObserver<T>.OnNext`, before the observer's `OnNext`).
    pub fn record_next(&self, value: &T) {
        let time = self.now_ticks();
        self.notifications.borrow_mut().push(Recorded::new(time, Notification::OnNext(value.clone())));
    }

    /// Records an error; the first one is kept unless the sequence had ended (`IObserver<T>.OnError`).
    pub fn record_error(&self, error: ObservableError) {
        let time = self.now_ticks();
        self.notifications.borrow_mut().push(Recorded::new(time, Notification::OnError(error.clone())));
        if !self.has_finalized() {
            *self.error.borrow_mut() = Some(error);
        }
        self.finalize();
    }

    /// Records the completion (`IObserver<T>.OnCompleted`).
    pub fn record_completed(&self) {
        let time = self.now_ticks();
        self.notifications.borrow_mut().push(Recorded::new(time, Notification::OnCompleted));
        self.has_completed.set(true);
        self.finalize();
    }
}
