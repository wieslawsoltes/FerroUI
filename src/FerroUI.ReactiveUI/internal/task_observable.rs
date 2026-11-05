//! `TaskObservable<T>` (`ReactiveUI.Core/Internal/TaskObservable.cs`), with the bridge it
//! subscribes through (`Shared/TaskObserverBridge.cs`) and its unit form
//! (`ReactiveUI.Shared/Internal/TaskUnitObservable.cs`).
//!
//! A task here is the [`Shared`] of a `Result<T, ObservableError>` (`ferroui_system::task`), and
//! the task without a result is that of a `Result<(), ObservableError>`, so `TaskUnitObservable`
//! (the `Task` form, which notifies `Unit`) is `TaskObservable<()>`.

use ferroui_reactive::disposables::Disposable;
use ferroui_system::task::Shared;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// A task as a sequence of its outcome (`TaskObservable<T>`): each subscription continues the
/// task, and the subscription does not remove the continuation, as upstream's does not.
pub struct TaskObservable<T> {
    task: Shared<Result<T, ObservableError>>,
}

impl<T: Clone + 'static> TaskObservable<T> {
    /// A sequence of the outcome of `task` (`new TaskObservable<T>(Task<T>)`).
    pub fn new(task: Shared<Result<T, ObservableError>>) -> Rc<Self> {
        Rc::new(Self { task })
    }
}

impl<T: Clone + 'static> IObservable<T> for TaskObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        subscribe(&self.task, observer)
    }
}

/// `TaskObserverBridge.Subscribe(Task<T>, IObserver<T>)`: when the task ends (at once if it has),
/// its result and the end go to the observer, or its error. A canceled task is the error of its
/// cancellation (`ferroui_system::task::Canceled` for the futures that stop on a token), and an
/// aborted or panicked one the error of [`JoinError::to_observable_error`](ferroui_system::task::JoinError::to_observable_error).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's error of a canceled task is a
/// `TaskCanceledException` naming the task; the task here has no id, and its own cancellation
/// error is forwarded.
pub(crate) fn subscribe<T: Clone + 'static>(task: &Shared<Result<T, ObservableError>>, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
    // `ContinueWith(..., ExecuteSynchronously)`: the continuation runs when the task ends, or at
    // once on a task that has ended.
    let _ = task.on_finished(move |outcome| match outcome {
        Ok(Ok(value)) => {
            observer.on_next(value.clone());
            observer.on_completed();
        }
        Ok(Err(error)) => observer.on_error(error.clone()),
        Err(error) => observer.on_error(error.to_observable_error()),
    });
    Disposable::empty()
}
