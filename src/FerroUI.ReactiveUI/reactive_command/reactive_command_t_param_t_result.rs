//! `ReactiveCommand<TParam, TResult>` (`ReactiveUI.Shared/ReactiveCommand/ReactiveCommand{TParam,TResult}.cs`):
//! a command whose executions are observables.
//!
//! The command holds the state of its streams (`CanExecute`, `IsExecuting`, `ThrownExceptions`
//! and the results) and the counts that drive them: the number of executions in flight and the
//! last value of the observable that gates it. Each execution is an [`Execution`]: it begins on
//! subscription, forwards the results of the execution observable to its subscriber and to the
//! results of the command, and ends when that observable ends, fails, or is disposed. The changes
//! of the streams are delivered on the output scheduler, inline when that is the immediate one.
//!
//! The gates of upstream (`lock (_gate)`) protect the counts against other threads; the command
//! here belongs to its thread, so the counts are cells, and no borrow is held while an observer
//! runs.

use super::i_command::ICommand;
use super::reactive_command_base::{icommand_execute, ReactiveCommandBase};
use super::{IReactiveCommand, IReactiveCommandOf};
use crate::internal::scheduler_extensions::SchedulerExtensions;
use crate::primitives::broadcaster::Broadcaster;
use crate::{IHandleObservableErrors, PanicException, RxSchedulers, RxState};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

/// How an execution is cancelled: `None` when disposing its subscription is enough (`NoCancel`
/// upstream), else the action that cancels the work it started (an asynchronous execution, which
/// keeps running until it ends).
pub(crate) type CancelAction = Option<Rc<dyn Fn()>>;

/// The results of one execution and how to cancel it (`(IObservable<TResult> Result, Action Cancel)`).
pub(crate) type ExecutionSource<R> = (Rc<dyn IObservable<R>>, CancelAction);

/// The function of a command: from a parameter, an observable of the execution to run
/// (`Func<TParam, IObservable<(IObservable<TResult> Result, Action Cancel)>>`). An `Err` is the
/// exception upstream catches when the function throws.
type ExecuteFn<P, R> = Rc<dyn Fn(P) -> Result<Rc<dyn IObservable<ExecutionSource<R>>>, ObservableError>>;

/// A reactive command with a parameter of type `P` and results of type `R`
/// (`ReactiveCommand<TParam, TResult>`). It is a shared handle: clones are the same command.
///
/// Its factories are the associated functions of `ReactiveCommand` (`ReactiveCommand::create`,
/// `create_from_observable`, `create_from_task`, `create_run_in_background`, `create_combined`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the parameter has a default (`P: Default`), which
/// `Execute()` and `ICommand.Execute(null)` pass where upstream passes `default(TParam)`; a command
/// whose parameter may be missing takes an `Option`.
pub struct ReactiveCommand<P, R> {
    inner: Rc<Inner<P, R>>,
}

impl<P, R> Clone for ReactiveCommand<P, R> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// Two handles are equal when they are the same command, as the default equality of .NET
/// compares references.
impl<P, R> PartialEq for ReactiveCommand<P, R> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<P, R> fmt::Debug for ReactiveCommand<P, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveCommand")
            .field("can_execute", &self.inner.can_execute_value.get())
            .field("is_executing", &self.inner.is_executing_value.get())
            .field("in_flight", &self.inner.in_flight.get())
            .finish()
    }
}

pub(crate) struct Inner<P, R> {
    /// The `ICommand` state (`ReactiveCommandBase`).
    base: Rc<ReactiveCommandBase>,
    /// `_outputScheduler`.
    output_scheduler: Rc<dyn IScheduler>,
    /// `_execute`.
    execute: ExecuteFn<P, R>,
    /// `_canExecuteSubscription`.
    can_execute_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_canExecuteBroadcaster`.
    can_execute_broadcaster: Broadcaster<bool>,
    /// `_isExecutingBroadcaster`.
    is_executing_broadcaster: Broadcaster<bool>,
    /// `_exceptionsBroadcaster`.
    exceptions_broadcaster: Broadcaster<ObservableError>,
    /// `_resultsBroadcaster`.
    results_broadcaster: Broadcaster<R>,
    /// `_userCanExecute`: the last value of the observable that gates the command.
    user_can_execute: Cell<bool>,
    /// `_canExecuteValue`.
    can_execute_value: Cell<bool>,
    /// `_isExecutingValue`.
    is_executing_value: Cell<bool>,
    /// `_inFlight`.
    in_flight: Cell<usize>,
    /// `_disposed`.
    disposed: Cell<bool>,
}

impl<P, R> Drop for Inner<P, R> {
    fn drop(&mut self) {
        // Nothing can reach the command any more: end its subscription to the gate, which holds
        // only a weak reference to it.
        if let Some(subscription) = self.can_execute_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> ReactiveCommand<P, R> {
    /// A command from a function that gives, for a parameter, the execution to run and how to
    /// cancel it (`protected internal ReactiveCommand(Func<TParam, IObservable<(IObservable<TResult>,
    /// Action)>>, IObservable<bool>?, IScheduler?)`). Without `can_execute` the command can always
    /// execute; without `output_scheduler` its streams notify on
    /// [`RxSchedulers::main_thread_scheduler`].
    pub(crate) fn from_cancellable(
        execute: impl Fn(P) -> Result<Rc<dyn IObservable<ExecutionSource<R>>>, ObservableError> + 'static,
        can_execute: Option<Rc<dyn IObservable<bool>>>,
        output_scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        let inner = Rc::new(Inner {
            base: ReactiveCommandBase::new(),
            output_scheduler: output_scheduler.unwrap_or_else(RxSchedulers::main_thread_scheduler),
            execute: Rc::new(execute),
            can_execute_subscription: RefCell::new(None),
            can_execute_broadcaster: Broadcaster::default(),
            is_executing_broadcaster: Broadcaster::default(),
            exceptions_broadcaster: Broadcaster::default(),
            results_broadcaster: Broadcaster::default(),
            user_can_execute: Cell::new(false),
            can_execute_value: Cell::new(false),
            is_executing_value: Cell::new(false),
            in_flight: Cell::new(0),
            disposed: Cell::new(false),
        });
        let can_execute = can_execute.unwrap_or_else(|| Observable::return_(true));
        let on_next: Weak<Inner<P, R>> = Rc::downgrade(&inner);
        let on_error = on_next.clone();
        let subscription = can_execute.subscribe(Observer::create_all(
            move |value| {
                if let Some(inner) = on_next.upgrade() {
                    on_user_can_execute(&inner, value);
                }
            },
            move |error| {
                if let Some(inner) = on_error.upgrade() {
                    on_user_can_execute_error(&inner, error);
                }
            },
            || {},
        ));
        *inner.can_execute_subscription.borrow_mut() = Some(subscription);
        Self { inner }
    }

    /// A command from a function that gives, for a parameter, the observable of the execution
    /// (`protected internal ReactiveCommand(Func<TParam, IObservable<TResult>>, IObservable<bool>?,
    /// IScheduler?)`); the execution is cancelled by disposing its subscription.
    pub(crate) fn from_observable(
        execute: impl Fn(P) -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
        can_execute: Option<Rc<dyn IObservable<bool>>>,
        output_scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        Self::from_cancellable(
            move |parameter| {
                let result = execute(parameter)?;
                let source: ExecutionSource<R> = (result, None);
                Ok(Observable::return_(source))
            },
            can_execute,
            output_scheduler,
        )
    }

    /// Whether the command can execute: the current value on subscription, then each change. It is
    /// false while an execution is in flight, and false after the gate failed (`CanExecute`).
    pub fn can_execute(&self) -> Rc<dyn IObservable<bool>> {
        self.can_execute_stream()
    }

    /// Whether the command is executing: the current value on subscription, then each change
    /// (`IsExecuting`).
    pub fn is_executing(&self) -> Rc<dyn IObservable<bool>> {
        self.is_executing_stream()
    }

    /// The errors of the executions and of the gate, on the output scheduler
    /// (`ThrownExceptions`). While nothing observes it, an error goes to
    /// [`RxState::default_exception_handler`].
    pub fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.thrown_exceptions_stream()
    }

    /// An observable that executes the command with `parameter` each time it is subscribed, and
    /// notifies the results of that execution (`Execute(TParam)`). Nothing runs until it is
    /// subscribed; disposing the subscription cancels the execution.
    pub fn execute(&self, parameter: P) -> Rc<dyn IObservable<R>> {
        self.execute_stream(parameter)
    }

    /// [`execute`](Self::execute) with the default parameter (`Execute()`).
    pub fn execute_default(&self) -> Rc<dyn IObservable<R>> {
        self.execute_stream(P::default())
    }

    /// The command as the observable of its results, which the operators take.
    pub fn as_observable(&self) -> Rc<dyn IObservable<R>> {
        Rc::new(self.clone())
    }

    fn can_execute_stream(&self) -> Rc<dyn IObservable<bool>> {
        Rc::new(BoolStream { owner: self.inner.clone(), is_executing: false })
    }

    fn is_executing_stream(&self) -> Rc<dyn IObservable<bool>> {
        Rc::new(BoolStream { owner: self.inner.clone(), is_executing: true })
    }

    fn thrown_exceptions_stream(&self) -> Rc<dyn IObservable<ObservableError>> {
        Rc::new(ExceptionStream { owner: self.inner.clone() })
    }

    fn execute_stream(&self, parameter: P) -> Rc<dyn IObservable<R>> {
        Rc::new(ExecuteObservable { owner: self.inner.clone(), parameter })
    }

    /// Ends the subscription to the gate (`Dispose(bool)`); executions in flight go on.
    fn dispose_command(&self) {
        self.inner.disposed.set(true);
        let subscription = self.inner.can_execute_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IObservable<R> for ReactiveCommand<P, R> {
    /// Notifies the results of every execution of the command (`Subscribe(IObserver<TResult>)`).
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let id = self.inner.results_broadcaster.add(observer);
        let owner = Rc::downgrade(&self.inner);
        Disposable::create(move || {
            if let Some(owner) = owner.upgrade() {
                owner.results_broadcaster.remove(id);
            }
        })
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IDisposable for ReactiveCommand<P, R> {
    fn dispose(&self) {
        self.dispose_command();
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IHandleObservableErrors for ReactiveCommand<P, R> {
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.thrown_exceptions_stream()
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IReactiveCommand for ReactiveCommand<P, R> {
    fn is_executing(&self) -> Rc<dyn IObservable<bool>> {
        self.is_executing_stream()
    }

    fn can_execute(&self) -> Rc<dyn IObservable<bool>> {
        self.can_execute_stream()
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IReactiveCommandOf<P, R> for ReactiveCommand<P, R> {
    fn execute(&self, parameter: P) -> Rc<dyn IObservable<R>> {
        self.execute_stream(parameter)
    }

    fn execute_default(&self) -> Rc<dyn IObservable<R>> {
        self.execute_stream(P::default())
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> ICommand for ReactiveCommand<P, R> {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.inner.base.icommand_can_execute(parameter)
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        icommand_execute::<P, R>(parameter, |parameter| self.execute_stream(parameter));
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.inner.base.add_can_execute_changed(handler)
    }
}

/// `OnUserCanExecute(bool)`.
fn on_user_can_execute<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>, value: bool) {
    if inner.disposed.get() {
        return;
    }
    inner.user_can_execute.set(value);
    let effective = value && inner.in_flight.get() == 0;
    let changed = effective != inner.can_execute_value.get();
    if !changed {
        return;
    }
    inner.can_execute_value.set(effective);
    inner.base.on_can_execute_changed(effective);
    inner.can_execute_broadcaster.next(effective);
}

/// `OnUserCanExecuteError(Exception)`.
fn on_user_can_execute_error<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>, error: ObservableError) {
    deliver_exception(inner, error);
    on_user_can_execute(inner, false);
}

/// `NotifyBegin()`.
fn notify_begin<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>) {
    let command = inner.clone();
    let _ = inner.output_scheduler.schedule_or_inline(move || apply_begin(&command));
}

/// `NotifyEnd()`.
fn notify_end<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>) {
    let command = inner.clone();
    let _ = inner.output_scheduler.schedule_or_inline(move || apply_end(&command));
}

/// `NotifyResult(TResult)`.
fn notify_result<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>, result: R) {
    let command = inner.clone();
    let _ = inner.output_scheduler.schedule_or_inline(move || command.results_broadcaster.next(result));
}

/// `DeliverException(Exception)`.
fn deliver_exception<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>, error: ObservableError) {
    let command = inner.clone();
    let _ = inner.output_scheduler.schedule_or_inline(move || apply_exception(&command, error));
}

/// `ApplyBegin()`.
fn apply_begin<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>) {
    let is_executing_changed = inner.in_flight.get() == 0;
    inner.in_flight.set(inner.in_flight.get() + 1);
    if is_executing_changed {
        inner.is_executing_value.set(true);
    }
    let can_execute_value = inner.user_can_execute.get() && inner.in_flight.get() == 0;
    let can_execute_changed = can_execute_value != inner.can_execute_value.get();
    if can_execute_changed {
        inner.can_execute_value.set(can_execute_value);
    }
    if is_executing_changed {
        inner.is_executing_broadcaster.next(true);
    }
    if !can_execute_changed {
        return;
    }
    inner.base.on_can_execute_changed(can_execute_value);
    inner.can_execute_broadcaster.next(can_execute_value);
}

/// `ApplyEnd()`.
fn apply_end<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>) {
    if inner.in_flight.get() > 0 {
        inner.in_flight.set(inner.in_flight.get() - 1);
    }
    let is_executing_changed = inner.in_flight.get() == 0 && inner.is_executing_value.get();
    if is_executing_changed {
        inner.is_executing_value.set(false);
    }
    let can_execute_value = inner.user_can_execute.get() && inner.in_flight.get() == 0;
    let can_execute_changed = can_execute_value != inner.can_execute_value.get();
    if can_execute_changed {
        inner.can_execute_value.set(can_execute_value);
    }
    if is_executing_changed {
        inner.is_executing_broadcaster.next(false);
    }
    if !can_execute_changed {
        return;
    }
    inner.base.on_can_execute_changed(can_execute_value);
    inner.can_execute_broadcaster.next(can_execute_value);
}

/// `ApplyException(Exception)`: to the observers of `ThrownExceptions`, or to the default
/// exception handler when there are none.
fn apply_exception<P: 'static, R: Clone + 'static>(inner: &Rc<Inner<P, R>>, error: ObservableError) {
    if inner.exceptions_broadcaster.has_observers() {
        inner.exceptions_broadcaster.next(error);
        return;
    }
    RxState::default_exception_handler().on_next(error);
}

/// `BoolStream`: `CanExecute` or `IsExecuting`, the current value on subscription, then each
/// change.
struct BoolStream<P, R> {
    owner: Rc<Inner<P, R>>,
    is_executing: bool,
}

impl<P: 'static, R: 'static> IObservable<bool> for BoolStream<P, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        // `AddBool`.
        let (id, current) = if self.is_executing {
            (self.owner.is_executing_broadcaster.add(observer.clone()), self.owner.is_executing_value.get())
        } else {
            (self.owner.can_execute_broadcaster.add(observer.clone()), self.owner.can_execute_value.get())
        };
        observer.on_next(current);
        // `BoolSubscription`, which `RemoveBool` ends.
        let owner = Rc::downgrade(&self.owner);
        let is_executing = self.is_executing;
        Disposable::create(move || {
            if let Some(owner) = owner.upgrade() {
                if is_executing {
                    owner.is_executing_broadcaster.remove(id);
                } else {
                    owner.can_execute_broadcaster.remove(id);
                }
            }
        })
    }
}

/// `ExceptionStream`: `ThrownExceptions`.
struct ExceptionStream<P, R> {
    owner: Rc<Inner<P, R>>,
}

impl<P: 'static, R: 'static> IObservable<ObservableError> for ExceptionStream<P, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservableError>>) -> Rc<dyn IDisposable> {
        let id = self.owner.exceptions_broadcaster.add(observer);
        let owner = Rc::downgrade(&self.owner);
        Disposable::create(move || {
            if let Some(owner) = owner.upgrade() {
                owner.exceptions_broadcaster.remove(id);
            }
        })
    }
}

/// `ExecuteObservable`: each subscription is an [`Execution`] with the parameter.
struct ExecuteObservable<P, R> {
    owner: Rc<Inner<P, R>>,
    parameter: P,
}

impl<P: Clone + 'static, R: Clone + 'static> IObservable<R> for ExecuteObservable<P, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        Execution::new(self.owner.clone(), observer).run(self.parameter.clone())
    }
}

/// One execution of the command (`Execution`): it observes the execution observable the function
/// of the command gave, then the results of the execution.
struct Execution<P, R> {
    this: Weak<Execution<P, R>>,
    owner: Rc<Inner<P, R>>,
    downstream: Rc<dyn IObserver<R>>,
    /// `_outer`.
    outer: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_inner`.
    inner: RefCell<Option<Rc<dyn IDisposable>>>,
    /// `_cancel`: `None` until the execution observable notified, or when disposing is enough.
    cancel: RefCell<CancelAction>,
    /// `_endFired`.
    end_fired: Cell<bool>,
    /// Whether the execution was disposed; a subscription made after that is disposed at once.
    disposed: Cell<bool>,
}

impl<P: Clone + 'static, R: Clone + 'static> Execution<P, R> {
    fn new(owner: Rc<Inner<P, R>>, downstream: Rc<dyn IObserver<R>>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner,
            downstream,
            outer: RefCell::new(None),
            inner: RefCell::new(None),
            cancel: RefCell::new(None),
            end_fired: Cell::new(false),
            disposed: Cell::new(false),
        })
    }

    fn this(&self) -> Rc<Self> {
        self.this.upgrade().expect("an execution is alive while it runs")
    }

    /// `Run(TParam)`.
    fn run(self: Rc<Self>, parameter: P) -> Rc<dyn IDisposable> {
        notify_begin(&self.owner);
        // `try { _outer = _execute(parameter).Subscribe(this); } catch (Exception ex)`: the
        // function of the command fails with an `Err`, or panics.
        let source = match PanicException::catch(|| (self.owner.execute)(parameter)) {
            Ok(source) => source,
            Err((error, _)) => Err(error),
        };
        match source {
            Ok(source) => {
                let observer: Rc<dyn IObserver<ExecutionSource<R>>> = self.clone();
                let subscription = source.subscribe(observer);
                self.store(&self.outer, subscription);
                self
            }
            Err(error) => {
                self.end_once();
                deliver_exception(&self.owner, error.clone());
                self.downstream.on_error(error);
                Disposable::empty()
            }
        }
    }

    /// Keeps a subscription, or disposes it at once when the execution was disposed.
    fn store(&self, slot: &RefCell<Option<Rc<dyn IDisposable>>>, subscription: Rc<dyn IDisposable>) {
        if self.disposed.get() {
            subscription.dispose();
            return;
        }
        *slot.borrow_mut() = Some(subscription);
    }

    /// `EndOnce()`.
    fn end_once(&self) {
        if self.end_fired.replace(true) {
            return;
        }
        notify_end(&self.owner);
    }

    /// `OnResult(TResult)`.
    fn on_result(&self, value: R) {
        self.downstream.on_next(value.clone());
        notify_result(&self.owner, value);
    }

    /// `OnResultCompleted()`.
    fn on_result_completed(&self) {
        self.end_once();
        self.downstream.on_completed();
    }
}

impl<P: Clone + 'static, R: Clone + 'static> IObserver<ExecutionSource<R>> for Execution<P, R> {
    fn on_next(&self, value: ExecutionSource<R>) {
        let (result, cancel) = value;
        *self.cancel.borrow_mut() = cancel;
        let subscription = result.subscribe(Rc::new(ResultObserver { execution: self.this() }));
        self.store(&self.inner, subscription);
    }

    fn on_error(&self, error: ObservableError) {
        self.end_once();
        deliver_exception(&self.owner, error.clone());
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {}
}

impl<P: Clone + 'static, R: Clone + 'static> IDisposable for Execution<P, R> {
    fn dispose(&self) {
        self.disposed.set(true);
        // Cancel first, while the cancellation source is still alive: disposing the inner
        // subscription tears down the asynchronous run.
        let cancel = self.cancel.borrow().clone();
        if let Some(cancel) = &cancel {
            cancel();
        }
        let inner = self.inner.borrow_mut().take();
        if let Some(inner) = inner {
            inner.dispose();
        }
        let outer = self.outer.borrow_mut().take();
        if let Some(outer) = outer {
            outer.dispose();
        }
        // For asynchronous executions the cancelled work keeps running until it ends, and
        // IsExecuting stays true until then: its terminal notification raises the balancing end.
        // For the other executions disposal ends the execution here.
        if cancel.is_some() {
            return;
        }
        self.end_once();
    }
}

/// `Execution.ResultObserver`.
struct ResultObserver<P, R> {
    execution: Rc<Execution<P, R>>,
}

impl<P: Clone + 'static, R: Clone + 'static> IObserver<R> for ResultObserver<P, R> {
    fn on_next(&self, value: R) {
        self.execution.on_result(value);
    }

    fn on_error(&self, error: ObservableError) {
        IObserver::<ExecutionSource<R>>::on_error(&*self.execution, error);
    }

    fn on_completed(&self) {
        self.execution.on_result_completed();
    }
}
