//! `TaskObservable<T>` (`ReactiveUI.Core/Internal/TaskObservable.cs`), with the bridge it
//! subscribes through (`Shared/TaskObserverBridge.cs`) and its unit form
//! (`ReactiveUI.Shared/Internal/TaskUnitObservable.cs`).
//!
//! A task here is `Task<T>` of `ferroui-system`, and the task without a result is `Task<()>`, so
//! `TaskUnitObservable` (the `Task` form, which notifies `Unit`) is `TaskObservable<()>`.

use ferroui_reactive::disposables::Disposable;
use ferroui_system::threading::tasks::{Task, TaskCanceledError};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// A task as a sequence of its outcome (`TaskObservable<T>`): each subscription continues the
/// task, and the subscription does not remove the continuation, as upstream's does not.
pub struct TaskObservable<T> {
    task: Task<T>,
}

impl<T: Clone + 'static> TaskObservable<T> {
    /// A sequence of the outcome of `task` (`new TaskObservable<T>(Task<T>)`).
    pub fn new(task: Task<T>) -> Rc<Self> {
        Rc::new(Self { task })
    }
}

impl<T: Clone + 'static> IObservable<T> for TaskObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        subscribe(&self.task, observer)
    }
}

/// `TaskObserverBridge.Subscribe(Task<T>, IObserver<T>)`: when the task ends (at once if it has),
/// its result and the end go to the observer, or its error; a canceled task is an error that is a
/// [`TaskCanceledError`].
pub(crate) fn subscribe<T: Clone + 'static>(task: &Task<T>, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
    let forward = move |completed: &Task<T>| {
        if completed.is_canceled() {
            observer.on_error(Rc::new(TaskCanceledError { task_id: Some(completed.id()) }) as ObservableError);
            return;
        }
        match completed.result() {
            Ok(value) => {
                observer.on_next(value);
                observer.on_completed();
            }
            Err(error) => observer.on_error(error),
        }
    };
    // `ContinueWith(..., ExecuteSynchronously)`: the continuation runs when the task ends, or at
    // once on a task that has ended.
    let _ = task.continue_with(forward);
    Disposable::empty()
}
