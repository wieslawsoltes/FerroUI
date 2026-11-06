//! `TaskObservableExtensions` (`Threading/Tasks/TaskObservableExtensions.cs`): a task as a
//! sequence and a sequence as a task; `StartAsync` and `FromAsync` (`Linq/QueryLanguage.Async.cs`),
//! which run a future as a task (docs/porting/reactive.md, section 5.8).
//!
//! Deviation (DEVIATIONS.md, Reactive): upstream's `Task<T>` is, as an argument, a future of
//! `Result<T, ObservableError>`, which runs as a task of the thread
//! (`ferroui_system::task::spawn_local`) whose outcome is shared ([`Shared`]); as a result, a
//! `Shared<Result<T, ObservableError>>`, whose continuations
//! ([`on_finished`](Shared::on_finished)) run synchronously before its awaiters are woken
//! (`ExecuteSynchronously`), and whose [`flatten`](Shared::flatten) is a future of the result. The
//! forms of a `Task` without a result are the same methods with `T = ()`. `TaskObservationOptions`
//! reduces to the scheduler (`_on`): `IgnoreExceptionsAfterUnsubscribe` concerns the unobserved
//! exceptions of the task scheduler of .NET, which tasks here do not have.

use crate::concurrency::{IScheduler, ImmediateScheduler, SchedulerExt};
use crate::disposables::{CancellationDisposable, SingleAssignmentDisposableValue, StableCompositeDisposable};
use crate::internal::TaskCompletion;
use crate::linq::{Observable, SequenceError};
use crate::ObservableBase;
use ferroui_system::task::{Flatten, JoinError, OnFinishedId, Shared};
use ferroui_system::threading::{CancellationGuard, CancellationToken};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

/// The outcome of an ended task as one result: an abort or a panic of the task that ran the future
/// is its error ([`JoinError::to_observable_error`]).
pub(crate) fn shared_result<T: Clone>(outcome: &Result<Result<T, ObservableError>, JoinError>) -> Result<T, ObservableError> {
    match outcome {
        Ok(Ok(value)) => Ok(value.clone()),
        Ok(Err(error)) => Err(error.clone()),
        Err(error) => Err(error.to_observable_error()),
    }
}

/// The outcome of an ended task, to its observer (`EmitTaskResult`).
fn emit_task_result<T>(result: Result<T, ObservableError>, observer: &Rc<dyn IObserver<T>>) {
    match result {
        Ok(value) => {
            observer.on_next(value);
            observer.on_completed();
        }
        Err(error) => observer.on_error(error),
    }
}

/// The `Shared` of a deprecated `Task`, which is used as it is (its continuations, and the
/// cancellation error that names it, are those of that task).
#[allow(deprecated)]
fn deprecated_task_shared<T: 'static>(slot: &mut dyn Any) -> Option<Shared<Result<T, ObservableError>>> {
    slot.downcast_mut::<Option<ferroui_system::threading::tasks::Task<T>>>().and_then(Option::take).map(Shared::from)
}

/// A future as the [`Shared`] of the task that runs it: a [`Flatten`] is the `Shared` it awaits,
/// used as it is (as upstream uses the task a function returns, so that the continuations run on
/// that task's end), as is a deprecated `Task`; another future runs as a task of the current thread
/// (`ferroui_system::task::spawn_local`), whose outcome is shared.
pub(crate) fn future_to_shared<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(future: Fut) -> Shared<Result<T, ObservableError>> {
    let mut slot = Some(future);
    if let Some(flatten) = (&mut slot as &mut dyn Any).downcast_mut::<Option<Flatten<T>>>() {
        if let Some(flatten) = flatten.take() {
            return flatten.into_shared();
        }
    }
    if let Some(shared) = deprecated_task_shared::<T>(&mut slot) {
        return shared;
    }
    spawn_shared(slot.take().expect("the future is in its slot"))
}

/// Runs `future` as a task of the current thread and shares its outcome (`Task.Run` of an `async`
/// function, the task of `StartAsync`).
pub(crate) fn spawn_shared<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(future: Fut) -> Shared<Result<T, ObservableError>> {
    ferroui_system::task::spawn_local(future).shared()
}

/// A task that has not ended, as a sequence (`SlowTaskObservable<TResult>`): each subscription
/// continues the task, on `scheduler` when there is one.
struct SlowTaskObservable<T> {
    task: Shared<Result<T, ObservableError>>,
    scheduler: Option<Rc<dyn IScheduler>>,
}

/// Removes the continuation of a subscription (the `CancellationDisposable` of upstream, whose
/// token cancels the continuation).
struct RemoveContinuation<T> {
    task: Shared<Result<T, ObservableError>>,
    id: Cell<Option<OnFinishedId>>,
}

impl<T: 'static> IDisposable for RemoveContinuation<T> {
    fn dispose(&self) {
        if let Some(id) = self.id.take() {
            self.task.remove_on_finished(id);
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for SlowTaskObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let id = match &self.scheduler {
            None => self.task.on_finished(move |outcome| emit_task_result(shared_result(outcome), &observer)),
            Some(scheduler) => {
                let scheduler = scheduler.clone();
                self.task.on_finished(move |outcome| {
                    let result = shared_result(outcome);
                    scheduler.schedule_action(move || emit_task_result(result, &observer));
                })
            }
        };
        Rc::new(RemoveContinuation { task: self.task.clone(), id: Cell::new(id) })
    }
}

/// `ToObservableImpl`: an ended task is `Return` or `Throw` on the scheduler (immediate by
/// default), another one a [`SlowTaskObservable`].
pub(crate) fn task_to_observable<T: Clone + 'static>(
    task: Shared<Result<T, ObservableError>>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<T>> {
    if let Some(result) = task.result() {
        let scheduler = scheduler.unwrap_or_else(ImmediateScheduler::instance);
        return match result {
            Ok(value) => Observable::return_on(value, scheduler),
            Err(error) => Observable::throw_on(error, scheduler),
        };
    }
    Rc::new(SlowTaskObservable { task, scheduler })
}

/// A task as a sequence (`TaskObservableExtensions.ToObservable`): any future of a
/// `Result<T, ObservableError>`, which runs as a task of the current thread; a
/// [`Flatten`] (`shared.flatten()`) is the `Shared` it awaits, used as it is.
pub trait TaskObservableExt<T> {
    /// A sequence of the result of the task, or of its error; a cancellation ends it with the
    /// error of the cancellation (`ToObservable(Task<TResult>)`, `ToObservable(Task)` for `T = ()`).
    fn to_observable(self) -> Rc<dyn IObservable<T>>;

    /// [`to_observable`](Self::to_observable) with the notifications on `scheduler`
    /// (`ToObservable(Task<TResult>, IScheduler)`, `ToObservable(Task<TResult>, TaskObservationOptions)`).
    fn to_observable_on(self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>;
}

impl<T: Clone + 'static, F: Future<Output = Result<T, ObservableError>> + 'static> TaskObservableExt<T> for F {
    fn to_observable(self) -> Rc<dyn IObservable<T>> {
        task_to_observable(future_to_shared(self), None)
    }

    fn to_observable_on(self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        task_to_observable(future_to_shared(self), Some(scheduler))
    }
}

/// The observer of `ToTask` (`ToTaskObserver<TResult>`): the last value ends the task when the
/// sequence ends, an error faults it, the cancellation of the token cancels it.
struct ToTaskObserver<T> {
    source: TaskCompletion<T>,
    registration: RefCell<Option<CancellationGuard>>,
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

/// `ToTask(IObservable<TResult>, CancellationToken)`: the `Shared` of the last value.
pub(crate) fn to_task<T: 'static>(source: Rc<dyn IObservable<T>>, token: CancellationToken) -> Shared<Result<T, ObservableError>> {
    let completion = TaskCompletion::new();
    let observer = Rc::new(ToTaskObserver {
        source: completion.clone(),
        registration: RefCell::new(None),
        last_value: RefCell::new(None),
        resource: SingleAssignmentDisposableValue::new(),
        is_stopped: Cell::new(false),
    });
    if token.can_be_cancelled() {
        // The registration holds the observer, as upstream's does: a source that keeps no
        // reference to its observer (`Never`) still ends the task when the token is canceled. The
        // end of the sequence disposes the registration.
        let target = observer.clone();
        let registration = token.register_local(move || target.cancel());
        *observer.registration.borrow_mut() = Some(registration);
    }
    let subscription = source.subscribe(observer.clone());
    observer.resource.set_disposable(Some(subscription));
    completion.shared()
}

/// `ContinueOnScheduler`: a task that ends as `task` does, on `scheduler`.
pub(crate) fn continue_on_scheduler<T: Clone + 'static>(
    task: Shared<Result<T, ObservableError>>,
    scheduler: Rc<dyn IScheduler>,
) -> Shared<Result<T, ObservableError>> {
    let completion = TaskCompletion::new();
    let result = completion.shared();
    task.on_finished(move |outcome| {
        let outcome = shared_result(outcome);
        scheduler.schedule_action(move || {
            completion.try_send(outcome);
        });
    });
    result
}

/// `StartAsyncImpl(Func<Task<TSource>>, options)`.
pub(crate) fn start_async<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    future: Fut,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<T>> {
    task_to_observable(spawn_shared(future), scheduler)
}

/// `StartAsyncImpl(Func<CancellationToken, Task<TSource>>, options)`: the subscription also cancels
/// the token given to the function (`StartAsyncObservable<TSource>`).
pub(crate) fn start_async_with_token<T: Clone + 'static, Fut: Future<Output = Result<T, ObservableError>> + 'static>(
    function_async: impl FnOnce(CancellationToken) -> Fut,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<T>> {
    let cancellable = CancellationDisposable::new();
    let result = task_to_observable(spawn_shared(function_async(cancellable.token())), scheduler);
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
        /// values faults it with `SequenceError::NoElements` (`ToTask(IObservable<TResult>)`). The
        /// task is the [`Shared`](ferroui_system::task::Shared) of its result, for several
        /// awaiters (`flatten()` is a future of the result), whose continuations
        /// (`on_finished`) run when the sequence ends, before its awaiters resume.
        fn to_task(&self) -> ferroui_system::task::Shared<Result<T, ferroui_system::ObservableError>> {
            crate::linq::task_observable::to_task(self.observable(), ferroui_system::threading::CancellationToken::none())
        }

        /// [`to_task`](Self::to_task) that the cancellation of `token` cancels, disposing the
        /// subscription: the task ends with the error `Canceled`
        /// (`ToTask(IObservable<TResult>, CancellationToken)`).
        fn to_task_with_token(
            &self,
            token: ferroui_system::threading::CancellationToken,
        ) -> ferroui_system::task::Shared<Result<T, ferroui_system::ObservableError>> {
            crate::linq::task_observable::to_task(self.observable(), token)
        }

        /// [`to_task`](Self::to_task) whose task ends on `scheduler`
        /// (`ToTask(IObservable<TResult>, IScheduler)`).
        fn to_task_on(
            &self,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> ferroui_system::task::Shared<Result<T, ferroui_system::ObservableError>>
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
        ) -> ferroui_system::task::Shared<Result<T, ferroui_system::ObservableError>>
        where
            T: Clone,
        {
            let task = crate::linq::task_observable::to_task(self.observable(), token);
            crate::linq::task_observable::continue_on_scheduler(task, scheduler)
        }
    };
}
pub(crate) use task_observable_methods;
