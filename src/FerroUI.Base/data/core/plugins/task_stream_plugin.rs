use super::IStreamPlugin;
use crate::data::core::WeakValue;
use crate::data::{BindingError, BindingErrorType, BindingNotification};
use crate::reactive::{IObservable, LightweightSubject, Observable};
use crate::threading::{Dispatcher, DispatcherPriority, FerroSynchronizationContext};
use crate::{BoxedValue, PropertyValue};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

#[derive(Clone)]
enum TaskStatus {
    Running,
    RanToCompletion(Option<BoxedValue>),
    Faulted(BindingError),
}

struct TaskShared {
    status: RefCell<TaskStatus>,
    continuations: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl TaskShared {
    fn new(status: TaskStatus) -> Rc<Self> {
        Rc::new(Self { status: RefCell::new(status), continuations: RefCell::new(Vec::new()) })
    }

    fn try_complete(&self, status: TaskStatus) -> bool {
        if !matches!(*self.status.borrow(), TaskStatus::Running) {
            return false;
        }
        self.status.replace(status);
        let continuations = std::mem::take(&mut *self.continuations.borrow_mut());
        for continuation in continuations {
            continuation();
        }
        true
    }
}

/// A task as a value that can be stored in a property and streamed by a
/// binding with the `^` operator: the handle to the eventual outcome of an
/// asynchronous operation, which is an untyped result or an error. It has
/// identity equality.
///
/// The outcome is produced by a [`TaskValueSource`], by a future running on
/// the dispatcher ([`TaskValue::run`]) or is known up front
/// ([`TaskValue::from_result`], [`TaskValue::from_exception`]).
#[derive(Clone)]
pub struct TaskValue(Rc<TaskShared>);

impl TaskValue {
    /// A task that has completed with `value`.
    pub fn from_result<T: PropertyValue>(value: T) -> Self {
        Self::from_untyped_result(Some(Rc::new(value)))
    }

    /// A task that has completed with an untyped value: null, a boxed value
    /// or a shared model object.
    pub fn from_untyped_result(value: Option<BoxedValue>) -> Self {
        Self(TaskShared::new(TaskStatus::RanToCompletion(value)))
    }

    /// A task that has failed with `error`.
    pub fn from_exception(error: BindingError) -> Self {
        Self(TaskShared::new(TaskStatus::Faulted(error)))
    }

    /// Runs `future` on the dispatcher of the calling thread and returns the
    /// task for its outcome. A future that is dropped before it completes
    /// (the dispatcher shut down, the future panicked) leaves the task
    /// running forever.
    pub fn run<F>(future: F) -> Self
    where
        F: Future<Output = Result<Option<BoxedValue>, BindingError>> + 'static,
    {
        let task = Self(TaskShared::new(TaskStatus::Running));
        let shared = task.0.clone();
        drop(Dispatcher::current_dispatcher().invoke_async_task_local(move || async move {
            let status = match future.await {
                Ok(value) => TaskStatus::RanToCompletion(value),
                Err(error) => TaskStatus::Faulted(error),
            };
            shared.try_complete(status);
        }));
        task
    }

    /// Whether the task has finished: completed or failed.
    pub fn is_completed(&self) -> bool {
        !matches!(*self.0.status.borrow(), TaskStatus::Running)
    }

    /// Whether the task ran to completion.
    pub fn is_completed_successfully(&self) -> bool {
        matches!(*self.0.status.borrow(), TaskStatus::RanToCompletion(_))
    }

    /// Whether the task failed.
    pub fn is_faulted(&self) -> bool {
        matches!(*self.0.status.borrow(), TaskStatus::Faulted(_))
    }

    /// The outcome of the task, if it has finished.
    pub fn result(&self) -> Option<Result<Option<BoxedValue>, BindingError>> {
        match self.0.status.borrow().clone() {
            TaskStatus::Running => None,
            TaskStatus::RanToCompletion(value) => Some(Ok(value)),
            TaskStatus::Faulted(error) => Some(Err(error)),
        }
    }

    /// Runs `continuation` when the task finishes; right away if it has
    /// finished already. The continuation runs where the task is completed:
    /// a caller that needs it on the dispatcher posts from it.
    pub fn continue_with(&self, continuation: impl FnOnce() + 'static) {
        if self.is_completed() {
            continuation();
        } else {
            self.0.continuations.borrow_mut().push(Box::new(continuation));
        }
    }
}

impl PartialEq for TaskValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The producer side of a [`TaskValue`] that is not bound to a future: the
/// equivalent of a task completion source.
pub struct TaskValueSource {
    task: TaskValue,
}

impl Default for TaskValueSource {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskValueSource {
    pub fn new() -> Self {
        Self { task: TaskValue(TaskShared::new(TaskStatus::Running)) }
    }

    /// The task completed by this source.
    pub fn task(&self) -> TaskValue {
        self.task.clone()
    }

    /// Completes the task with `value`. Panics if the task has already
    /// finished.
    pub fn set_result<T: PropertyValue>(&self, value: T) {
        self.set_untyped_result(Some(Rc::new(value)));
    }

    /// Completes the task with an untyped value. Panics if the task has
    /// already finished.
    pub fn set_untyped_result(&self, value: Option<BoxedValue>) {
        if !self.try_set_untyped_result(value) {
            panic!("An attempt was made to transition a task to a final state when it had already completed.");
        }
    }

    /// Fails the task with `error`. Panics if the task has already finished.
    pub fn set_exception(&self, error: BindingError) {
        if !self.try_set_exception(error) {
            panic!("An attempt was made to transition a task to a final state when it had already completed.");
        }
    }

    /// Completes the task with an untyped value unless it has already
    /// finished.
    pub fn try_set_untyped_result(&self, value: Option<BoxedValue>) -> bool {
        self.task.0.try_complete(TaskStatus::RanToCompletion(value))
    }

    /// Fails the task with `error` unless it has already finished.
    pub fn try_set_exception(&self, error: BindingError) -> bool {
        self.task.0.try_complete(TaskStatus::Faulted(error))
    }
}

/// Handles binding to [`TaskValue`]s for the `^` stream binding operator.
///
/// A completed task produces its result, a failed one a binding error
/// notification. The outcome of a task that is still running is delivered
/// from a dispatcher job queued when the task finishes.
pub struct TaskStreamPlugin;

impl TaskStreamPlugin {
    /// The stream of a finished task: its result, or a binding error
    /// notification for a failed one. Panics if the task is still running.
    pub fn handle_completed(task: &TaskValue) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        match task.result() {
            Some(Ok(value)) => Observable::return_(value),
            Some(Err(error)) => Observable::return_(Some(
                Rc::new(BindingNotification::with_error(error, BindingErrorType::Error)) as BoxedValue,
            )),
            None => panic!("handle_completed called for non-completed task."),
        }
    }
}

impl IStreamPlugin for TaskStreamPlugin {
    fn match_(&self, reference: &WeakValue) -> bool {
        reference.upgrade().is_some_and(|t| t.is::<TaskValue>())
    }

    fn start(&self, reference: &WeakValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>> {
        let target = reference.upgrade();
        let Some(task) = target.as_ref().and_then(|t| t.downcast_ref::<TaskValue>()) else {
            return Some(Observable::empty());
        };
        Some(Self::observe(task))
    }
}

impl TaskStreamPlugin {
    /// The stream of a task: the outcome right away if the task has
    /// finished, otherwise delivered from a dispatcher job queued when the
    /// task finishes.
    pub fn observe(task: &TaskValue) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        if task.is_completed() {
            return Self::handle_completed(task);
        }

        // The continuation runs where the current synchronization context
        // posts to: a job of this thread's dispatcher.
        let subject = LightweightSubject::<Option<BoxedValue>>::new();
        let dispatcher = Dispatcher::current_dispatcher();
        let priority = FerroSynchronizationContext::current().map_or(DispatcherPriority::NORMAL, |c| c.priority());
        let completed = task.clone();
        let observer = subject.clone();
        task.continue_with(move || {
            dispatcher.clone().post_local(
                move || {
                    Self::handle_completed(&completed).subscribe(Rc::new(observer));
                },
                priority,
            );
        });
        Rc::new(subject)
    }
}
