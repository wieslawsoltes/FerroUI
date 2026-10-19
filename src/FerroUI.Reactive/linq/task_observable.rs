//! `TaskObservableExtensions` (`Threading/Tasks/TaskObservableExtensions.cs`): a task as a
//! sequence and a sequence as a task; `StartAsync` and `FromAsync` (`Linq/QueryLanguage.Async.cs`),
//! which run a future as a task (docs/porting/reactive.md, section 5.8).
//!
//! Deviation (DEVIATIONS.md, Reactive): upstream's functions return a `Task<T>`; here they return
//! a future of `Result<T, ObservableError>`, which runs as a task of the thread
//! (`Task::spawn_local`). A `Task<T>` is such a future. The forms of a `Task` without a result are
//! the same methods with `T = ()`. `TaskObservationOptions` reduces to the scheduler (`_on`):
//! `IgnoreExceptionsAfterUnsubscribe` concerns the unobserved exceptions of the task scheduler of
//! .NET, which tasks here do not have.

use crate::concurrency::{IScheduler, ImmediateScheduler, SchedulerExt};
use crate::disposables::{CancellationDisposable, SingleAssignmentDisposableValue, StableCompositeDisposable};
use crate::linq::{Observable, SequenceError};
use crate::ObservableBase;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::threading::{CancellationToken, LocalCancellationTokenRegistration};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

/// The outcome of an ended task, to its observer (`EmitTaskResult`).
fn emit_task_result<T: Clone + 'static>(task: &Task<T>, observer: &Rc<dyn IObserver<T>>) {
    match task.result() {
        Ok(value) => {
            observer.on_next(value);
            observer.on_completed();
        }
        Err(error) => observer.on_error(error),
    }
}

/// A task that has not ended, as a sequence (`SlowTaskObservable<TResult>`): each subscription
/// continues the task, on `scheduler` when there is one.
struct SlowTaskObservable<T> {
    task: Task<T>,
    scheduler: Option<Rc<dyn IScheduler>>,
}

/// Removes the continuation of a subscription (the `CancellationDisposable` of upstream, whose
/// token cancels the continuation).
struct RemoveContinuation<T> {
    task: Task<T>,
    id: Cell<Option<ferroui_system::threading::tasks::ContinuationId>>,
}

impl<T: 'static> IDisposable for RemoveContinuation<T> {
    fn dispose(&self) {
        if let Some(id) = self.id.take() {
            self.task.remove_continuation(id);
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for SlowTaskObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let id = match &self.scheduler {
            None => self.task.continue_with(move |task| emit_task_result(task, &observer)),
            Some(scheduler) => {
                let scheduler = scheduler.clone();
                self.task.continue_with(move |task| {
                    let task = task.clone();
                    scheduler.schedule_action(move || emit_task_result(&task, &observer));
                })
            }
        };
        Rc::new(RemoveContinuation { task: self.task.clone(), id: Cell::new(id) })
    }
}

/// `ToObservableImpl`: an ended task is `Return` or `Throw` on the scheduler (immediate by
/// default), another one a [`SlowTaskObservable`].
pub(crate) fn task_to_observable<T: Clone + 'static>(task: Task<T>, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<dyn IObservable<T>> {
    if task.is_completed() {
        let scheduler = scheduler.unwrap_or_else(ImmediateScheduler::instance);
        return match task.result() {
            Ok(value) => Observable::return_on(value, scheduler),
            Err(error) => Observable::throw_on(error, scheduler),
        };
    }
    Rc::new(SlowTaskObservable { task, scheduler })
}

/// A task as a sequence (`TaskObservableExtensions.ToObservable`).
pub trait TaskObservableExt<T> {
    /// A sequence of the result of the task, or of its error; a canceled task ends it with a
    /// `TaskCanceledError` (`ToObservable(Task<TResult>)`, `ToObservable(Task)` for `T = ()`).
    fn to_observable(&self) -> Rc<dyn IObservable<T>>;

    /// [`to_observable`](Self::to_observable) with the notifications on `scheduler`
    /// (`ToObservable(Task<TResult>, IScheduler)`, `ToObservable(Task<TResult>, TaskObservationOptions)`).
    fn to_observable_on(&self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>;
}

impl<T: Clone + 'static> TaskObservableExt<T> for Task<T> {
    fn to_observable(&self) -> Rc<dyn IObservable<T>> {
        task_to_observable(self.clone(), None)
    }

    fn to_observable_on(&self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        task_to_observable(self.clone(), Some(scheduler))
    }
}

/// The observer of `ToTask` (`ToTaskObserver<TResult>`): the last value ends the task when the
/// sequence ends, an error faults it, the cancellation of the token cancels it.
struct ToTaskObserver<T> {
    source: TaskCompletionSource<T>,
    registration: RefCell<Option<LocalCancellationTokenRegistration>>,
    last_value: RefCell<Option<T>>,
    resource: SingleAssignmentDisposableValue,
    is_stopped: Cell<bool>,
}

impl<T: 'static> ToTaskObserver<T> {
    fn dispose_registration(&self) {
        let registration = self.registration.borrow_mut().take();
        if let Some(registration) = registration {
            registration.dispose();
        }
    }

    /// `Cancel`.
    fn cancel(&self) {
        self.resource.dispose();
        self.source.try_set_canceled();
    }
}

impl<T: 'static> IObserver<T> for ToTaskObserver<T> {
    fn on_next(&self, value: T) {
        if self.is_stopped.get() {
            return;
        }
        *self.last_value.borrow_mut() = Some(value);
    }

    fn on_error(&self, error: ObservableError) {
        if self.is_stopped.replace(true) {
            return;
        }
        self.source.try_set_exception(error);
        self.dispose_registration();
        self.resource.dispose();
    }

    fn on_completed(&self) {
        if self.is_stopped.replace(true) {
            return;
        }
        let last = self.last_value.borrow_mut().take();
        match last {
            Some(value) => {
                self.source.try_set_result(value);
            }
            None => {
                self.source.try_set_exception(Rc::new(SequenceError::NoElements));
            }
        }
        self.dispose_registration();
        self.resource.dispose();
    }
}

/// `ToTask(IObservable<TResult>, CancellationToken)`.
pub(crate) fn to_task<T: 'static>(source: Rc<dyn IObservable<T>>, token: CancellationToken) -> Task<T> {
    let completion = TaskCompletionSource::new();
    let observer = Rc::new(ToTaskObserver {
        source: completion.clone(),
        registration: RefCell::new(None),
        last_value: RefCell::new(None),
        resource: SingleAssignmentDisposableValue::new(),
        is_stopped: Cell::new(false),
    });
    if token.can_be_canceled() {
        // The registration holds the observer, as upstream's does: a source that keeps no
        // reference to its observer (`Never`) still ends the task when the token is canceled. The
        // end of the sequence disposes the registration.
        let target = observer.clone();
        let registration = token.register_local(move || target.cancel());
        *observer.registration.borrow_mut() = Some(registration);
    }
    let subscription = source.subscribe(observer.clone());
    observer.resource.set_disposable(Some(subscription));
    completion.task()
}

/// `ContinueOnScheduler`: a task that ends as `task` does, on `scheduler`.
pub(crate) fn continue_on_scheduler<T: Clone + 'static>(task: Task<T>, scheduler: Rc<dyn IScheduler>) -> Task<T> {
    let completion = TaskCompletionSource::new();
    let result = completion.task();
    task.continue_with(move |task| {
        let task = task.clone();
        scheduler.schedule_action(move || {
            if task.is_canceled() {
                completion.try_set_canceled();
            } else {
                match task.result() {
                    Ok(value) => {
                        completion.try_set_result(value);
                    }
                    Err(error) => {
                        completion.try_set_exception(error);
                    }
                }
            }
        });
    });
    result
}

/// `StartAsyncImpl(Func<Task<TSource>>, options)`.
pub(crate) fn start_async<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    future: Fut,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<T>> {
    task_to_observable(Task::spawn_local(future), scheduler)
}

/// `StartAsyncImpl(Func<CancellationToken, Task<TSource>>, options)`: the subscription also cancels
/// the token given to the function (`StartAsyncObservable<TSource>`).
pub(crate) fn start_async_with_token<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    function_async: impl FnOnce(CancellationToken) -> Fut,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<T>> {
    let cancellable = CancellationDisposable::new();
    let result = task_to_observable(Task::spawn_local(function_async(cancellable.token())), scheduler);
    Rc::new(ObservableBase::from_fn(move |observer: Rc<dyn IObserver<T>>| {
        let subscription = result.subscribe(observer);
        let cancellable: Rc<dyn IDisposable> = cancellable.clone();
        let composite: Rc<dyn IDisposable> = StableCompositeDisposable::create(cancellable, subscription);
        composite
    }))
}

impl Observable {
    /// Runs the future the function gives at once, as a task, and gives a sequence of its outcome:
    /// a hot sequence, shared by its subscriptions (`StartAsync(Func<Task<TResult>>)`,
    /// `StartAsync(Func<Task>)` for `T = ()`).
    pub fn start_async<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl FnOnce() -> Fut,
    ) -> Rc<dyn IObservable<T>> {
        start_async(function_async(), None)
    }

    /// [`start_async`](Self::start_async) with the notifications on `scheduler`
    /// (`StartAsync(Func<Task<TResult>>, TaskObservationOptions)`).
    pub fn start_async_on<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl FnOnce() -> Fut,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        start_async(function_async(), Some(scheduler))
    }

    /// [`start_async`](Self::start_async) with a cancellation token, which disposing a subscription
    /// cancels (`StartAsync(Func<CancellationToken, Task<TResult>>)`).
    pub fn start_async_with_token<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl FnOnce(CancellationToken) -> Fut,
    ) -> Rc<dyn IObservable<T>> {
        start_async_with_token(function_async, None)
    }

    /// [`start_async_with_token`](Self::start_async_with_token) with the notifications on
    /// `scheduler` (`StartAsync(Func<CancellationToken, Task<TResult>>, TaskObservationOptions)`).
    pub fn start_async_with_token_on<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl FnOnce(CancellationToken) -> Fut,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        start_async_with_token(function_async, Some(scheduler))
    }

    /// Runs the future the function gives for each subscription, as a task, and gives a sequence
    /// of its outcome (`FromAsync(Func<Task<TResult>>)`, `FromAsync(Func<Task>)` for `T = ()`):
    /// `Defer` of `StartAsync`.
    pub fn from_async<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl Fn() -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || start_async(function_async(), None))
    }

    /// [`from_async`](Self::from_async) with the notifications on `scheduler`
    /// (`FromAsync(Func<Task<TResult>>, TaskObservationOptions)`).
    pub fn from_async_on<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl Fn() -> Fut + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || start_async(function_async(), Some(scheduler.clone())))
    }

    /// [`from_async`](Self::from_async) with a cancellation token, which disposing the
    /// subscription cancels (`FromAsync(Func<CancellationToken, Task<TResult>>)`).
    pub fn from_async_with_token<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl Fn(CancellationToken) -> Fut + 'static,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || start_async_with_token(&function_async, None))
    }

    /// [`from_async_with_token`](Self::from_async_with_token) with the notifications on
    /// `scheduler` (`FromAsync(Func<CancellationToken, Task<TResult>>, TaskObservationOptions)`).
    pub fn from_async_with_token_on<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
        function_async: impl Fn(CancellationToken) -> Fut + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Observable::defer(move || start_async_with_token(&function_async, Some(scheduler.clone())))
    }
}

macro_rules! task_observable_methods {
    () => {
        /// A task that ends with the last value of the sequence, or its error; a sequence without
        /// values faults it with `SequenceError::NoElements` (`ToTask(IObservable<TResult>)`).
        fn to_task(&self) -> ferroui_system::threading::tasks::Task<T> {
            crate::linq::task_observable::to_task(self.observable(), ferroui_system::threading::CancellationToken::none())
        }

        /// [`to_task`](Self::to_task) that the cancellation of `token` cancels, disposing the
        /// subscription (`ToTask(IObservable<TResult>, CancellationToken)`).
        fn to_task_with_token(&self, token: ferroui_system::threading::CancellationToken) -> ferroui_system::threading::tasks::Task<T> {
            crate::linq::task_observable::to_task(self.observable(), token)
        }

        /// [`to_task`](Self::to_task) whose task ends on `scheduler`
        /// (`ToTask(IObservable<TResult>, IScheduler)`).
        fn to_task_on(&self, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> ferroui_system::threading::tasks::Task<T>
        where
            T: Clone,
        {
            let task = crate::linq::task_observable::to_task(self.observable(), ferroui_system::threading::CancellationToken::none());
            crate::linq::task_observable::continue_on_scheduler(task, scheduler)
        }

        /// [`to_task_with_token`](Self::to_task_with_token) whose task ends on `scheduler`
        /// (`ToTask(IObservable<TResult>, CancellationToken, IScheduler)`).
        fn to_task_with_token_on(
            &self,
            token: ferroui_system::threading::CancellationToken,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> ferroui_system::threading::tasks::Task<T>
        where
            T: Clone,
        {
            let task = crate::linq::task_observable::to_task(self.observable(), token);
            crate::linq::task_observable::continue_on_scheduler(task, scheduler)
        }
    };
}
pub(crate) use task_observable_methods;
