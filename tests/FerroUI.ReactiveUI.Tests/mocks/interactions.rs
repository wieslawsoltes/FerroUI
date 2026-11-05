//! The objects of the tests of interactions: the scheduler the test drives by hand
//! (ReactiveUI.Binding.Tests, `TestModels/ManualSequencer.cs`) and the observer that completes a
//! task (`InteractionTests.CompletionObserver<T>`).

use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler, ScheduledAction};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::task::{Canceled, Shared, SharedSender};
use ferroui_system::{IDisposable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// A scheduler that queues its work until the test runs it (`ManualSequencer`); delayed work is
/// queued as the rest.
pub struct ManualScheduler {
    this: Weak<ManualScheduler>,
    pending: RefCell<VecDeque<ScheduledAction>>,
}

impl ManualScheduler {
    /// The most work [`run_until_idle`](Self::run_until_idle) runs (`MaxItems`).
    const MAX_ITEMS: usize = 1_000;

    /// A scheduler with nothing queued.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), pending: RefCell::new(VecDeque::new()) })
    }

    /// The scheduler as the handle the operators take.
    pub fn scheduler(self: &Rc<Self>) -> Rc<dyn IScheduler> {
        self.clone()
    }

    fn run_one(&self) -> bool {
        let Some(action) = self.pending.borrow_mut().pop_front() else { return false };
        let scheduler: Rc<dyn IScheduler> = self.this.upgrade().expect("the scheduler is alive while it runs");
        let _ = action(&scheduler);
        true
    }

    /// Runs the work queued now, and returns how much ran (`RunPending()`).
    pub fn run_pending(&self) -> usize {
        let count = self.pending.borrow().len();
        for _ in 0..count {
            self.run_one();
        }
        count
    }

    /// Runs the work queued, and the work it queues, until none is left; returns the number of
    /// passes, or `None` when more than `MaxItems` ran or `max_passes` did not suffice
    /// (`RunUntilIdle(int)`, which returns -1).
    pub fn run_until_idle(&self, max_passes: usize) -> Option<usize> {
        let mut ran = 0;
        for pass in 0..max_passes {
            let count = self.pending.borrow().len();
            if count == 0 {
                return Some(pass);
            }
            for _ in 0..count {
                ran += 1;
                if ran > Self::MAX_ITEMS {
                    return None;
                }
                self.run_one();
            }
        }
        None
    }
}

impl IScheduler for ManualScheduler {
    fn now(&self) -> SystemTime {
        ImmediateScheduler::instance().now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.pending.borrow_mut().push_back(action);
        Disposable::empty()
    }

    fn schedule_after(&self, _due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }

    fn schedule_at(&self, _due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }
}

/// An observer that completes a task with the last value when its sequence ends, or faults it
/// with the error (`InteractionTests.CompletionObserver<T>`).
pub struct CompletionObserver<T> {
    sender: RefCell<Option<SharedSender<Result<T, ObservableError>>>>,
    completion: Shared<Result<T, ObservableError>>,
    last: RefCell<Option<T>>,
}

impl<T: Clone + 'static> CompletionObserver<T> {
    /// An observer whose task has not ended.
    pub fn new() -> Rc<Self> {
        let (sender, completion) = Shared::channel();
        Rc::new(Self { sender: RefCell::new(Some(sender)), completion, last: RefCell::new(None) })
    }

    /// The task (`Completion`).
    pub fn completion(&self) -> Shared<Result<T, ObservableError>> {
        self.completion.clone()
    }

    /// `TrySetResult`, `TrySetException`, `TrySetCanceled`: the first end ends the task.
    fn try_send(&self, result: Result<T, ObservableError>) {
        // Taken out of the cell before it sends: the continuations may reach the observer.
        let sender = self.sender.borrow_mut().take();
        if let Some(sender) = sender {
            sender.send(result);
        }
    }
}

impl<T: Clone + 'static> IObserver<T> for CompletionObserver<T> {
    fn on_next(&self, value: T) {
        *self.last.borrow_mut() = Some(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.try_send(Err(error));
    }

    fn on_completed(&self) {
        let last = self.last.borrow_mut().take();
        match last {
            Some(value) => self.try_send(Ok(value)),
            None => self.try_send(Err(Rc::new(Canceled) as ObservableError)),
        }
    }
}
