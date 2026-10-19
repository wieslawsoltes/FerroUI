//! `ReactiveCommand` (`ReactiveUI.Shared/ReactiveCommand/ReactiveCommand.cs`): the factories of
//! reactive commands.
//!
//! Upstream they are the static methods of the non-generic class `ReactiveCommand`, overloaded on
//! the delegate they take and on the optional `canExecute` and `outputScheduler`. Here they are
//! associated functions of `ReactiveCommand` (`ReactiveCommand::create(..)`), one name per delegate
//! shape:
//!
//! | Upstream | Here |
//! |---|---|
//! | `Create(Action)`, `Create(Func<TResult>)` | [`create`](ReactiveCommand::create) |
//! | `Create(Action<TParam>)`, `Create(Func<TParam, TResult>)` | [`create_param`](ReactiveCommand::create_param) |
//! | `CreateRunInBackground(...)` | [`create_run_in_background`](ReactiveCommand::create_run_in_background), [`create_run_in_background_param`](ReactiveCommand::create_run_in_background_param) |
//! | `CreateFromObservable(...)` | [`create_from_observable`](ReactiveCommand::create_from_observable), [`create_from_observable_param`](ReactiveCommand::create_from_observable_param) |
//! | `CreateFromTask(Func<Task<TResult>>)`, `CreateFromTask(Func<Task>)` | [`create_from_task`](ReactiveCommand::create_from_task) |
//! | `CreateFromTask(Func<TParam, Task<TResult>>)`, `CreateFromTask(Func<TParam, Task>)` | [`create_from_task_param`](ReactiveCommand::create_from_task_param) |
//! | `CreateFromTask(Func<CancellationToken, Task<TResult>>)`, ... | [`create_from_task_cancellable`](ReactiveCommand::create_from_task_cancellable) |
//! | `CreateFromTask(Func<TParam, CancellationToken, Task<TResult>>)`, ... | [`create_from_task_param_cancellable`](ReactiveCommand::create_from_task_param_cancellable) |
//! | `CreateCombined(...)` | [`create_combined`](ReactiveCommand::create_combined) |
//!
//! Each has a `_with` form that takes `can_execute` and the output scheduler (and, for the
//! background forms, the background scheduler); `None` is upstream's `null`. An `Action` is a
//! function returning `()`, and a `Task` without a result is a future of `()`, so each pair of
//! overloads is one function.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI): the exception of a function goes to the execution and
//! to `ThrownExceptions` upstream. Here the `try_` form of a factory takes a function returning a
//! `Result`, whose `Err` takes that path; a panic of the function of any form is caught where
//! upstream catches the exception, and takes it as a [`PanicException`](crate::PanicException). A
//! task is a closure returning a future (a `Task<R>` is one), run as a task of the thread when the
//! execution is subscribed (docs/porting/reactive.md, section 5.8). The null checks of the
//! arguments (`ArgumentNullException`) have no counterpart: the arguments cannot be null.

use super::combined_reactive_command::CombinedReactiveCommand;
use super::reactive_command_t_param_t_result::{ExecutionSource, ReactiveCommand};
use super::IReactiveCommandOf;
use crate::internal::task_observable::TaskObservable;
use crate::mixins::observable_mixins::{from_async_with_all_notifications, from_async_with_all_notifications_param, AsyncExecution};
use crate::primitives::start_signal::StartSignal;
use crate::RxSchedulers;
use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactive::ObservableExt;
use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IObservable, ObservableError};
use std::future::Future;
use std::rc::Rc;

/// The observable that gates a command: `None` when it can always execute.
pub type OptionalCanExecute = Option<Rc<dyn IObservable<bool>>>;

/// A scheduler argument: `None` for the default one.
pub type OptionalScheduler = Option<Rc<dyn IScheduler>>;

/// The execution observable of an asynchronous execution as a command runs it.
fn cancellable_source<R: 'static>(executions: Rc<dyn IObservable<AsyncExecution<R>>>) -> Rc<dyn IObservable<ExecutionSource<R>>> {
    executions.select(|(result, cancel)| {
        let source: ExecutionSource<R> = (result, Some(cancel));
        source
    })
}

impl ReactiveCommand<(), ()> {
    // ---- Create ----------------------------------------------------------------------------

    /// A command without a parameter that runs `execute` and notifies what it returns
    /// (`Create(Action)`, `Create<TResult>(Func<TResult>)`).
    pub fn create<R: Clone + 'static>(execute: impl Fn() -> R + 'static) -> ReactiveCommand<(), R> {
        Self::create_with(execute, None, None)
    }

    /// [`create`](Self::create) with the observable that gates the command and the scheduler its
    /// streams notify on (`Create(Action, IObservable<bool>?, IScheduler?)` and the overloads
    /// with one of the two).
    pub fn create_with<R: Clone + 'static>(
        execute: impl Fn() -> R + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        Self::try_create_with(move || Ok(execute()), can_execute, output_scheduler)
    }

    /// [`create`](Self::create) with a function that may fail; its error goes to the execution and
    /// to `ThrownExceptions` (`Create` with a function that throws).
    pub fn try_create<R: Clone + 'static>(execute: impl Fn() -> Result<R, ObservableError> + 'static) -> ReactiveCommand<(), R> {
        Self::try_create_with(execute, None, None)
    }

    /// [`try_create`](Self::try_create) with the gate and the output scheduler.
    pub fn try_create_with<R: Clone + 'static>(
        execute: impl Fn() -> Result<R, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        let execute = Rc::new(execute);
        ReactiveCommand::from_observable(
            move |_: ()| {
                let execute = execute.clone();
                let source: Rc<dyn IObservable<R>> = StartSignal::new(move || execute(), ImmediateScheduler::instance());
                Ok(source)
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command with a parameter that runs `execute` with it and notifies what it returns
    /// (`Create<TParam>(Action<TParam>)`, `Create<TParam, TResult>(Func<TParam, TResult>)`).
    pub fn create_param<P: Clone + Default + 'static, R: Clone + 'static>(execute: impl Fn(P) -> R + 'static) -> ReactiveCommand<P, R> {
        Self::create_param_with(execute, None, None)
    }

    /// [`create_param`](Self::create_param) with the gate and the output scheduler.
    pub fn create_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> R + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_param_with(move |parameter| Ok(execute(parameter)), can_execute, output_scheduler)
    }

    /// [`create_param`](Self::create_param) with a function that may fail.
    pub fn try_create_param<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<R, ObservableError> + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_param_with(execute, None, None)
    }

    /// [`try_create_param`](Self::try_create_param) with the gate and the output scheduler.
    pub fn try_create_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<R, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        let execute = Rc::new(execute);
        ReactiveCommand::from_observable(
            move |parameter: P| {
                let execute = execute.clone();
                let source: Rc<dyn IObservable<R>> = StartSignal::new(move || execute(parameter.clone()), ImmediateScheduler::instance());
                Ok(source)
            },
            can_execute,
            output_scheduler,
        )
    }

    // ---- CreateRunInBackground ---------------------------------------------------------------

    /// A command without a parameter that runs `execute` on the background scheduler
    /// ([`RxSchedulers::task_pool_scheduler`] by default) and notifies what it returns
    /// (`CreateRunInBackground(Action)`, `CreateRunInBackground<TResult>(Func<TResult>)`).
    pub fn create_run_in_background<R: Clone + 'static>(execute: impl Fn() -> R + 'static) -> ReactiveCommand<(), R> {
        Self::create_run_in_background_with(execute, None, None, None)
    }

    /// [`create_run_in_background`](Self::create_run_in_background) with the gate, the background
    /// scheduler and the output scheduler (`CreateRunInBackground(Action, IObservable<bool>?,
    /// IScheduler?, IScheduler?)` and the overloads with some of them).
    pub fn create_run_in_background_with<R: Clone + 'static>(
        execute: impl Fn() -> R + 'static,
        can_execute: OptionalCanExecute,
        background_scheduler: OptionalScheduler,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        Self::try_create_run_in_background_with(move || Ok(execute()), can_execute, background_scheduler, output_scheduler)
    }

    /// [`create_run_in_background`](Self::create_run_in_background) with a function that may fail.
    pub fn try_create_run_in_background<R: Clone + 'static>(
        execute: impl Fn() -> Result<R, ObservableError> + 'static,
    ) -> ReactiveCommand<(), R> {
        Self::try_create_run_in_background_with(execute, None, None, None)
    }

    /// [`try_create_run_in_background`](Self::try_create_run_in_background) with the gate, the
    /// background scheduler and the output scheduler.
    pub fn try_create_run_in_background_with<R: Clone + 'static>(
        execute: impl Fn() -> Result<R, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        background_scheduler: OptionalScheduler,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        let execute = Rc::new(execute);
        Self::create_from_observable_with(
            move || {
                let execute = execute.clone();
                let scheduler = background_scheduler.clone().unwrap_or_else(RxSchedulers::task_pool_scheduler);
                let source: Rc<dyn IObservable<R>> = StartSignal::new(move || execute(), scheduler);
                source
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command with a parameter that runs `execute` with it on the background scheduler
    /// (`CreateRunInBackground<TParam>(Action<TParam>)`,
    /// `CreateRunInBackground<TParam, TResult>(Func<TParam, TResult>)`).
    pub fn create_run_in_background_param<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> R + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::create_run_in_background_param_with(execute, None, None, None)
    }

    /// [`create_run_in_background_param`](Self::create_run_in_background_param) with the gate, the
    /// background scheduler and the output scheduler.
    pub fn create_run_in_background_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> R + 'static,
        can_execute: OptionalCanExecute,
        background_scheduler: OptionalScheduler,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_run_in_background_param_with(
            move |parameter| Ok(execute(parameter)),
            can_execute,
            background_scheduler,
            output_scheduler,
        )
    }

    /// [`create_run_in_background_param`](Self::create_run_in_background_param) with a function
    /// that may fail.
    pub fn try_create_run_in_background_param<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<R, ObservableError> + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_run_in_background_param_with(execute, None, None, None)
    }

    /// [`try_create_run_in_background_param`](Self::try_create_run_in_background_param) with the
    /// gate, the background scheduler and the output scheduler.
    pub fn try_create_run_in_background_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<R, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        background_scheduler: OptionalScheduler,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        let execute = Rc::new(execute);
        Self::create_from_observable_param_with(
            move |parameter: P| {
                let execute = execute.clone();
                let scheduler = background_scheduler.clone().unwrap_or_else(RxSchedulers::task_pool_scheduler);
                let source: Rc<dyn IObservable<R>> = StartSignal::new(move || execute(parameter.clone()), scheduler);
                source
            },
            can_execute,
            output_scheduler,
        )
    }

    // ---- CreateCombined ----------------------------------------------------------------------

    /// A command that executes every child command with its parameter, and notifies the list of
    /// their results (`CreateCombined<TParam, TResult>(IEnumerable<ReactiveCommandBase<TParam, TResult>>)`).
    ///
    /// # Panics
    /// When there is no child command (`ArgumentException`).
    pub fn create_combined<P, R, C>(child_commands: impl IntoIterator<Item = C>) -> CombinedReactiveCommand<P, R>
    where
        P: Clone + Default + 'static,
        R: Clone + 'static,
        C: IReactiveCommandOf<P, R> + 'static,
    {
        Self::create_combined_with(child_commands, None, None)
    }

    /// [`create_combined`](Self::create_combined) with the observable that gates the combined
    /// command and the output scheduler.
    pub fn create_combined_with<P, R, C>(
        child_commands: impl IntoIterator<Item = C>,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> CombinedReactiveCommand<P, R>
    where
        P: Clone + Default + 'static,
        R: Clone + 'static,
        C: IReactiveCommandOf<P, R> + 'static,
    {
        let children = child_commands.into_iter().map(|child| {
            let child: Rc<dyn IReactiveCommandOf<P, R>> = Rc::new(child);
            child
        });
        CombinedReactiveCommand::new(children, can_execute, output_scheduler)
    }

    // ---- CreateFromObservable ----------------------------------------------------------------

    /// A command without a parameter whose execution is the observable `execute` gives
    /// (`CreateFromObservable<TResult>(Func<IObservable<TResult>>)`).
    pub fn create_from_observable<R: Clone + 'static>(execute: impl Fn() -> Rc<dyn IObservable<R>> + 'static) -> ReactiveCommand<(), R> {
        Self::create_from_observable_with(execute, None, None)
    }

    /// [`create_from_observable`](Self::create_from_observable) with the gate and the output
    /// scheduler.
    pub fn create_from_observable_with<R: Clone + 'static>(
        execute: impl Fn() -> Rc<dyn IObservable<R>> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        Self::try_create_from_observable_with(move || Ok(execute()), can_execute, output_scheduler)
    }

    /// [`create_from_observable`](Self::create_from_observable) with a function that may fail; its
    /// error fails the execution.
    pub fn try_create_from_observable<R: Clone + 'static>(
        execute: impl Fn() -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
    ) -> ReactiveCommand<(), R> {
        Self::try_create_from_observable_with(execute, None, None)
    }

    /// [`try_create_from_observable`](Self::try_create_from_observable) with the gate and the
    /// output scheduler.
    pub fn try_create_from_observable_with<R: Clone + 'static>(
        execute: impl Fn() -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        ReactiveCommand::from_observable(move |_: ()| execute(), can_execute, output_scheduler)
    }

    /// A command with a parameter whose execution is the observable `execute` gives for it
    /// (`CreateFromObservable<TParam, TResult>(Func<TParam, IObservable<TResult>>)`).
    pub fn create_from_observable_param<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Rc<dyn IObservable<R>> + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::create_from_observable_param_with(execute, None, None)
    }

    /// [`create_from_observable_param`](Self::create_from_observable_param) with the gate and the
    /// output scheduler.
    pub fn create_from_observable_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Rc<dyn IObservable<R>> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_from_observable_param_with(move |parameter| Ok(execute(parameter)), can_execute, output_scheduler)
    }

    /// [`create_from_observable_param`](Self::create_from_observable_param) with a function that
    /// may fail.
    pub fn try_create_from_observable_param<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::try_create_from_observable_param_with(execute, None, None)
    }

    /// [`try_create_from_observable_param`](Self::try_create_from_observable_param) with the gate
    /// and the output scheduler.
    pub fn try_create_from_observable_param_with<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        ReactiveCommand::from_observable(execute, can_execute, output_scheduler)
    }

    // ---- CreateFromTask ----------------------------------------------------------------------

    /// A command without a parameter whose execution runs the future `execute` gives, as a task
    /// of the thread, and notifies its result (`CreateFromTask<TResult>(Func<Task<TResult>>)`,
    /// `CreateFromTask(Func<Task>)` for `R = ()`). Disposing the execution does not stop the task.
    pub fn create_from_task<R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn() -> Fut + 'static,
    ) -> ReactiveCommand<(), R> {
        Self::create_from_task_with(execute, None, None)
    }

    /// [`create_from_task`](Self::create_from_task) with the gate and the output scheduler.
    pub fn create_from_task_with<R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn() -> Fut + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        Self::create_from_observable_with(
            move || {
                let source: Rc<dyn IObservable<R>> = TaskObservable::new(Task::spawn_local(execute()));
                source
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command with a parameter whose execution runs the future `execute` gives for it
    /// (`CreateFromTask<TParam, TResult>(Func<TParam, Task<TResult>>)`,
    /// `CreateFromTask<TParam>(Func<TParam, Task>)` for `R = ()`).
    pub fn create_from_task_param<P: Clone + Default + 'static, R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn(P) -> Fut + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::create_from_task_param_with(execute, None, None)
    }

    /// [`create_from_task_param`](Self::create_from_task_param) with the gate and the output
    /// scheduler.
    pub fn create_from_task_param_with<P: Clone + Default + 'static, R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn(P) -> Fut + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        Self::create_from_observable_param_with(
            move |parameter| {
                let source: Rc<dyn IObservable<R>> = TaskObservable::new(Task::spawn_local(execute(parameter)));
                source
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command without a parameter whose execution runs the future `execute` gives for a
    /// cancellation token; disposing the execution cancels the token, and the command stays
    /// executing until the future ends (`CreateFromTask<TResult>(Func<CancellationToken,
    /// Task<TResult>>)`, `CreateFromTask(Func<CancellationToken, Task>)` for `R = ()`).
    pub fn create_from_task_cancellable<R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn(CancellationToken) -> Fut + 'static,
    ) -> ReactiveCommand<(), R> {
        Self::create_from_task_cancellable_with(execute, None, None)
    }

    /// [`create_from_task_cancellable`](Self::create_from_task_cancellable) with the gate and the
    /// output scheduler.
    pub fn create_from_task_cancellable_with<R: Clone + 'static, Fut: Future<Output = Result<R, ObservableError>> + 'static>(
        execute: impl Fn(CancellationToken) -> Fut + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<(), R> {
        let execute = Rc::new(execute);
        Self::create_from_observable_cancellable(
            move |_: ()| {
                let execute = execute.clone();
                cancellable_source(from_async_with_all_notifications(move |token| execute(token)))
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command with a parameter whose execution runs the future `execute` gives for it and a
    /// cancellation token (`CreateFromTask<TParam, TResult>(Func<TParam, CancellationToken,
    /// Task<TResult>>)`, `CreateFromTask<TParam>(Func<TParam, CancellationToken, Task>)` for
    /// `R = ()`).
    pub fn create_from_task_param_cancellable<
        P: Clone + Default + 'static,
        R: Clone + 'static,
        Fut: Future<Output = Result<R, ObservableError>> + 'static,
    >(
        execute: impl Fn(P, CancellationToken) -> Fut + 'static,
    ) -> ReactiveCommand<P, R> {
        Self::create_from_task_param_cancellable_with(execute, None, None)
    }

    /// [`create_from_task_param_cancellable`](Self::create_from_task_param_cancellable) with the
    /// gate and the output scheduler.
    pub fn create_from_task_param_cancellable_with<
        P: Clone + Default + 'static,
        R: Clone + 'static,
        Fut: Future<Output = Result<R, ObservableError>> + 'static,
    >(
        execute: impl Fn(P, CancellationToken) -> Fut + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        let execute = Rc::new(execute);
        Self::create_from_observable_cancellable(
            move |parameter: P| {
                let execute = execute.clone();
                cancellable_source(from_async_with_all_notifications_param(move |parameter, token| execute(parameter, token), parameter))
            },
            can_execute,
            output_scheduler,
        )
    }

    /// A command whose execution observable gives the results and the action that cancels them
    /// (`internal CreateFromObservableCancellable<TParam, TResult>(Func<TParam, IObservable<(IObservable<TResult>,
    /// Action)>>, ...)`; the form without a parameter is this one ignoring it).
    pub(crate) fn create_from_observable_cancellable<P: Clone + Default + 'static, R: Clone + 'static>(
        execute: impl Fn(P) -> Rc<dyn IObservable<ExecutionSource<R>>> + 'static,
        can_execute: OptionalCanExecute,
        output_scheduler: OptionalScheduler,
    ) -> ReactiveCommand<P, R> {
        ReactiveCommand::from_cancellable(move |parameter| Ok(execute(parameter)), can_execute, output_scheduler)
    }
}
