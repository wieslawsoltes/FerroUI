//! `Start` of a function or an action on a scheduler (`QueryLanguage.Async.cs`: `Start`, through
//! the `ToAsync` of a function without arguments). The forms of `Start` with a task belong to the
//! asynchronous conversions.

use crate::concurrency::{IScheduler, SchedulerDefaults, SchedulerExt};
use crate::subjects::AsyncSubject;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

impl crate::linq::Observable {
    /// Runs `function` on the scheduler of asynchronous conversions and notifies its result, to
    /// every observer, when it has run (`Start<TResult>(Func<TResult>)`).
    pub fn start<T: Clone + 'static>(function: impl FnOnce() -> T + 'static) -> Rc<dyn IObservable<T>> {
        Self::start_on(function, SchedulerDefaults::async_conversions())
    }

    /// Runs `function` on `scheduler` and notifies its result, to every observer, when it has run
    /// (`Start<TResult>(Func<TResult>, IScheduler)`).
    pub fn start_on<T: Clone + 'static>(function: impl FnOnce() -> T + 'static, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Self::try_start_on(move || Ok(function()), scheduler)
    }

    /// [`start`](Self::start) with a function that may fail; a failure is notified as the error of
    /// the sequence (`Start<TResult>(Func<TResult>)` with a function that throws).
    pub fn try_start<T: Clone + 'static>(function: impl FnOnce() -> Result<T, ObservableError> + 'static) -> Rc<dyn IObservable<T>> {
        Self::try_start_on(function, SchedulerDefaults::async_conversions())
    }

    /// [`start_on`](Self::start_on) with a function that may fail; a failure is notified as the
    /// error of the sequence (`Start<TResult>(Func<TResult>, IScheduler)` with a function that
    /// throws).
    pub fn try_start_on<T: Clone + 'static>(
        function: impl FnOnce() -> Result<T, ObservableError> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        let subject = AsyncSubject::<T>::new();
        let target = subject.clone();
        scheduler.schedule_action(move || match function() {
            Ok(result) => {
                target.on_next(result);
                target.on_completed();
            }
            Err(error) => target.on_error(error),
        });
        subject.as_observable()
    }

    /// Runs `action` on the scheduler of asynchronous conversions and notifies a unit value, to
    /// every observer, when it has run (`Start(Action)`).
    pub fn start_action(action: impl FnOnce() + 'static) -> Rc<dyn IObservable<()>> {
        Self::start_action_on(action, SchedulerDefaults::async_conversions())
    }

    /// Runs `action` on `scheduler` and notifies a unit value, to every observer, when it has run
    /// (`Start(Action, IScheduler)`).
    pub fn start_action_on(action: impl FnOnce() + 'static, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<()>> {
        Self::try_start_action_on(move || Ok(action()), scheduler)
    }

    /// [`start_action`](Self::start_action) with an action that may fail; a failure is notified as
    /// the error of the sequence (`Start(Action)` with an action that throws).
    pub fn try_start_action(action: impl FnOnce() -> Result<(), ObservableError> + 'static) -> Rc<dyn IObservable<()>> {
        Self::try_start_action_on(action, SchedulerDefaults::async_conversions())
    }

    /// [`start_action_on`](Self::start_action_on) with an action that may fail; a failure is
    /// notified as the error of the sequence (`Start(Action, IScheduler)` with an action that
    /// throws).
    pub fn try_start_action_on(action: impl FnOnce() -> Result<(), ObservableError> + 'static, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<()>> {
        let subject = AsyncSubject::<()>::new();
        let target = subject.clone();
        scheduler.schedule_action(move || match action() {
            Ok(()) => {
                target.on_next(());
                target.on_completed();
            }
            Err(error) => target.on_error(error),
        });
        subject.as_observable()
    }
}
