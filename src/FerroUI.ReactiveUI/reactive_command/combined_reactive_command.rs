//! `CombinedReactiveCommand<TParam, TResult>` (`ReactiveUI.Shared/ReactiveCommand/CombinedReactiveCommand.cs`):
//! a command that executes several child commands with one parameter and notifies the list of
//! their results.
//!
//! It wraps a [`ReactiveCommand`] whose gate is "the parent gate and every child can execute",
//! and whose execution combines the latest result of each child execution. Its `ThrownExceptions`
//! merges those of the children with the failure of the parent gate, each once.

use super::i_command::ICommand;
use super::reactive_command_base::{icommand_execute, ReactiveCommandBase};
use super::reactive_command_t_param_t_result::ReactiveCommand;
use super::{IReactiveCommand, IReactiveCommandOf};
use crate::IHandleObservableErrors;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// A command that executes child commands together (`CombinedReactiveCommand<TParam, TResult>`):
/// its result is the list of the results of the children. It is a shared handle: clones are the
/// same command.
pub struct CombinedReactiveCommand<P, R> {
    inner: Rc<CombinedInner<P, R>>,
}

impl<P, R> Clone for CombinedReactiveCommand<P, R> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<P, R> PartialEq for CombinedReactiveCommand<P, R> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<P, R> fmt::Debug for CombinedReactiveCommand<P, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CombinedReactiveCommand").field("inner_command", &self.inner.inner_command).finish()
    }
}

struct CombinedInner<P, R> {
    /// The `ICommand` state (`ReactiveCommandBase`).
    base: Rc<ReactiveCommandBase>,
    /// `_innerCommand`.
    inner_command: ReactiveCommand<P, Vec<R>>,
    /// `_canExecuteSubscription`.
    can_execute_subscription: Rc<dyn IDisposable>,
    /// `_innerExceptionsSubscription`.
    inner_exceptions_subscription: Rc<dyn IDisposable>,
    /// `_thrownExceptions`.
    thrown_exceptions: Rc<dyn IObservable<ObservableError>>,
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CombinedReactiveCommand<P, R> {
    /// A command that executes `child_commands` together, gated by `can_execute` (always
    /// executable when `None`) and every child, with its streams on `output_scheduler`
    /// ([`RxSchedulers::main_thread_scheduler`](crate::RxSchedulers::main_thread_scheduler) when
    /// `None`) (`protected internal CombinedReactiveCommand(IEnumerable<ReactiveCommandBase<TParam,
    /// TResult>>, IObservable<bool>?, IScheduler?)` and its two shorter overloads).
    ///
    /// # Panics
    /// When there is no child command (`ArgumentException: No child commands provided.`).
    pub fn new(
        child_commands: impl IntoIterator<Item = Rc<dyn IReactiveCommandOf<P, R>>>,
        can_execute: Option<Rc<dyn IObservable<bool>>>,
        output_scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        let children: Rc<[Rc<dyn IReactiveCommandOf<P, R>>]> = child_commands.into_iter().collect();
        if children.is_empty() {
            panic!("ArgumentException: No child commands provided. (Parameter 'childCommands')");
        }

        let parent_gate = can_execute.unwrap_or_else(|| Observable::return_(true));

        // All-true of [parent, child0..childN] (the parent gate plus every child's CanExecute).
        let mut can_execute_sources: Vec<Rc<dyn IObservable<bool>>> = Vec::with_capacity(children.len() + 1);
        can_execute_sources.push(parent_gate.clone());
        can_execute_sources.extend(children.iter().map(|child| child.can_execute()));
        let combined_can_execute: Rc<dyn IObservable<bool>> = Rc::new(AllTrueCanExecuteObservable { sources: can_execute_sources.into() });

        let execute_children = children.clone();
        let inner_command = ReactiveCommand::from_observable(
            move |parameter: P| {
                let executions: Rc<[Rc<dyn IObservable<R>>]> =
                    execute_children.iter().map(|child| child.execute(parameter.clone())).collect();
                let results: Rc<dyn IObservable<Vec<R>>> = Rc::new(CombinedResultsObservable { sources: executions });
                Ok(results)
            },
            Some(combined_can_execute),
            output_scheduler,
        );

        // The public exception stream surfaces each child command's exceptions plus any failure of
        // the parent gate, each exactly once. A child error also propagates through the inner
        // command (it aggregates the child executions), as does a parent-gate failure, so the
        // inner command's ThrownExceptions would re-report the same exceptions; observe and discard
        // it separately to keep them handled without double-counting.
        let mut exception_sources: Vec<Rc<dyn IObservable<ObservableError>>> = children.iter().map(|child| child.thrown_exceptions()).collect();
        exception_sources.push(Rc::new(CanExecuteFailureObservable { source: parent_gate }));
        let thrown_exceptions: Rc<dyn IObservable<ObservableError>> = Rc::new(MergedExceptionsObservable { sources: exception_sources.into() });
        let inner_exceptions_subscription = inner_command.thrown_exceptions().subscribe(Observer::create_all(|_| {}, |_| {}, || {}));

        let base = ReactiveCommandBase::new();
        let on_can_execute = base.clone();
        let can_execute_subscription = inner_command
            .can_execute()
            .subscribe(Observer::create_all(move |value| on_can_execute.on_can_execute_changed(value), |_| {}, || {}));

        Self {
            inner: Rc::new(CombinedInner { base, inner_command, can_execute_subscription, inner_exceptions_subscription, thrown_exceptions }),
        }
    }

    /// Whether the combined command can execute (`CanExecute`): the parent gate and every child
    /// can, and no execution is in flight.
    pub fn can_execute(&self) -> Rc<dyn IObservable<bool>> {
        self.inner.inner_command.can_execute()
    }

    /// Whether the combined command is executing (`IsExecuting`).
    pub fn is_executing(&self) -> Rc<dyn IObservable<bool>> {
        self.inner.inner_command.is_executing()
    }

    /// The errors of the children and of the parent gate (`ThrownExceptions`).
    pub fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.inner.thrown_exceptions.clone()
    }

    /// An observable that executes every child with `parameter` when it is subscribed, and
    /// notifies the list of their results (`Execute(TParam)`).
    pub fn execute(&self, parameter: P) -> Rc<dyn IObservable<Vec<R>>> {
        self.inner.inner_command.execute(parameter)
    }

    /// [`execute`](Self::execute) with the default parameter (`Execute()`).
    pub fn execute_default(&self) -> Rc<dyn IObservable<Vec<R>>> {
        self.inner.inner_command.execute_default()
    }

    /// The command as the observable of its results, which the operators take.
    pub fn as_observable(&self) -> Rc<dyn IObservable<Vec<R>>> {
        Rc::new(self.clone())
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IObservable<Vec<R>> for CombinedReactiveCommand<P, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<R>>>) -> Rc<dyn IDisposable> {
        self.inner.inner_command.subscribe(observer)
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IDisposable for CombinedReactiveCommand<P, R> {
    /// `Dispose(bool)`.
    fn dispose(&self) {
        self.inner.can_execute_subscription.dispose();
        self.inner.inner_exceptions_subscription.dispose();
        IDisposable::dispose(&self.inner.inner_command);
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IHandleObservableErrors for CombinedReactiveCommand<P, R> {
    fn thrown_exceptions(&self) -> Rc<dyn IObservable<ObservableError>> {
        self.inner.thrown_exceptions.clone()
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IReactiveCommand for CombinedReactiveCommand<P, R> {
    fn is_executing(&self) -> Rc<dyn IObservable<bool>> {
        self.inner.inner_command.is_executing()
    }

    fn can_execute(&self) -> Rc<dyn IObservable<bool>> {
        self.inner.inner_command.can_execute()
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> IReactiveCommandOf<P, Vec<R>> for CombinedReactiveCommand<P, R> {
    fn execute(&self, parameter: P) -> Rc<dyn IObservable<Vec<R>>> {
        self.inner.inner_command.execute(parameter)
    }

    fn execute_default(&self) -> Rc<dyn IObservable<Vec<R>>> {
        self.inner.inner_command.execute_default()
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> ICommand for CombinedReactiveCommand<P, R> {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.inner.base.icommand_can_execute(parameter)
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        let command = self.inner.inner_command.clone();
        icommand_execute::<P, Vec<R>>(parameter, move |parameter| command.execute(parameter));
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.inner.base.add_can_execute_changed(handler)
    }
}

/// The subscriptions of a sink to its sources, by index.
type Subscriptions = RefCell<Vec<Option<Rc<dyn IDisposable>>>>;

fn dispose_all(subscriptions: &Subscriptions) {
    let taken: Vec<_> = subscriptions.borrow_mut().iter_mut().map(Option::take).collect();
    for subscription in taken.into_iter().flatten() {
        subscription.dispose();
    }
}

/// `AllTrueCanExecuteObservable`: true when the latest value of every source is true, once every
/// source has notified one.
struct AllTrueCanExecuteObservable {
    sources: Rc<[Rc<dyn IObservable<bool>>]>,
}

impl IObservable<bool> for AllTrueCanExecuteObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(AllTrueSink {
            downstream: observer,
            latest: RefCell::new(vec![None; self.sources.len()]),
            done_count: Cell::new(0),
            stopped: Cell::new(false),
            subscriptions: RefCell::new(vec![None; self.sources.len()]),
        });
        for (index, source) in self.sources.iter().enumerate() {
            let subscription = source.subscribe(Rc::new(AllTrueElement { parent: sink.clone(), index }));
            sink.subscriptions.borrow_mut()[index] = Some(subscription);
        }
        sink
    }
}

struct AllTrueSink {
    downstream: Rc<dyn IObserver<bool>>,
    latest: RefCell<Vec<Option<bool>>>,
    done_count: Cell<usize>,
    stopped: Cell<bool>,
    subscriptions: Subscriptions,
}

impl AllTrueSink {
    fn on_next_at(&self, index: usize, value: bool) {
        if self.stopped.get() {
            return;
        }
        let all = {
            let mut latest = self.latest.borrow_mut();
            latest[index] = Some(value);
            if latest.iter().any(Option::is_none) {
                return;
            }
            latest.iter().all(|value| *value == Some(true))
        };
        self.downstream.on_next(all);
    }

    fn on_error_at(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_error(error);
    }

    fn on_completed_at(&self) {
        if self.stopped.get() {
            return;
        }
        self.done_count.set(self.done_count.get() + 1);
        if self.done_count.get() < self.latest.borrow().len() {
            return;
        }
        self.stopped.set(true);
        self.downstream.on_completed();
    }
}

impl IDisposable for AllTrueSink {
    fn dispose(&self) {
        dispose_all(&self.subscriptions);
    }
}

struct AllTrueElement {
    parent: Rc<AllTrueSink>,
    index: usize,
}

impl IObserver<bool> for AllTrueElement {
    fn on_next(&self, value: bool) {
        self.parent.on_next_at(self.index, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error_at(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed_at();
    }
}

/// `CombinedResultsObservable`: the list of the latest result of every source, each time one
/// notifies once every source has notified; it ends when every source has ended.
struct CombinedResultsObservable<R> {
    sources: Rc<[Rc<dyn IObservable<R>>]>,
}

impl<R: Clone + 'static> IObservable<Vec<R>> for CombinedResultsObservable<R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<R>>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(CombinedResultsSink {
            downstream: observer,
            latest: RefCell::new(vec![None; self.sources.len()]),
            done_count: Cell::new(0),
            stopped: Cell::new(false),
            subscriptions: RefCell::new(vec![None; self.sources.len()]),
        });
        for (index, source) in self.sources.iter().enumerate() {
            let subscription = source.subscribe(Rc::new(CombinedResultsElement { parent: sink.clone(), index }));
            sink.subscriptions.borrow_mut()[index] = Some(subscription);
        }
        sink
    }
}

struct CombinedResultsSink<R> {
    downstream: Rc<dyn IObserver<Vec<R>>>,
    latest: RefCell<Vec<Option<R>>>,
    done_count: Cell<usize>,
    stopped: Cell<bool>,
    subscriptions: Subscriptions,
}

impl<R: Clone> CombinedResultsSink<R> {
    fn on_next_at(&self, index: usize, value: R) {
        if self.stopped.get() {
            return;
        }
        let list = {
            let mut latest = self.latest.borrow_mut();
            latest[index] = Some(value);
            if latest.iter().any(Option::is_none) {
                return;
            }
            latest.iter().flatten().cloned().collect::<Vec<R>>()
        };
        self.downstream.on_next(list);
    }

    fn on_error_at(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_error(error);
    }

    fn on_completed_at(&self) {
        if self.stopped.get() {
            return;
        }
        self.done_count.set(self.done_count.get() + 1);
        if self.done_count.get() < self.latest.borrow().len() {
            return;
        }
        self.stopped.set(true);
        self.downstream.on_completed();
    }
}

impl<R> IDisposable for CombinedResultsSink<R> {
    fn dispose(&self) {
        dispose_all(&self.subscriptions);
    }
}

struct CombinedResultsElement<R> {
    parent: Rc<CombinedResultsSink<R>>,
    index: usize,
}

impl<R: Clone> IObserver<R> for CombinedResultsElement<R> {
    fn on_next(&self, value: R) {
        self.parent.on_next_at(self.index, value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error_at(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed_at();
    }
}

/// `CanExecuteFailureObservable`: the error of the gate as a value, then the end.
struct CanExecuteFailureObservable {
    source: Rc<dyn IObservable<bool>>,
}

impl IObservable<ObservableError> for CanExecuteFailureObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservableError>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(CanExecuteFailureSink { downstream: observer }))
    }
}

struct CanExecuteFailureSink {
    downstream: Rc<dyn IObserver<ObservableError>>,
}

impl IObserver<bool> for CanExecuteFailureSink {
    fn on_next(&self, _value: bool) {}

    fn on_error(&self, error: ObservableError) {
        self.downstream.on_next(error);
        self.downstream.on_completed();
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}

/// `MergedExceptionsObservable`: the errors of every source; it ends when every source has ended.
struct MergedExceptionsObservable {
    sources: Rc<[Rc<dyn IObservable<ObservableError>>]>,
}

impl IObservable<ObservableError> for MergedExceptionsObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservableError>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(MergedExceptionsSink {
            downstream: observer,
            count: self.sources.len(),
            done_count: Cell::new(0),
            stopped: Cell::new(false),
            subscriptions: RefCell::new(vec![None; self.sources.len()]),
        });
        for (index, source) in self.sources.iter().enumerate() {
            let subscription = source.subscribe(Rc::new(MergedExceptionsElement { parent: sink.clone() }));
            sink.subscriptions.borrow_mut()[index] = Some(subscription);
        }
        sink
    }
}

struct MergedExceptionsSink {
    downstream: Rc<dyn IObserver<ObservableError>>,
    count: usize,
    done_count: Cell<usize>,
    stopped: Cell<bool>,
    subscriptions: Subscriptions,
}

impl MergedExceptionsSink {
    fn on_next_at(&self, value: ObservableError) {
        if self.stopped.get() {
            return;
        }
        self.downstream.on_next(value);
    }

    fn on_error_at(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.downstream.on_error(error);
    }

    fn on_completed_at(&self) {
        if self.stopped.get() {
            return;
        }
        self.done_count.set(self.done_count.get() + 1);
        if self.done_count.get() < self.count {
            return;
        }
        self.stopped.set(true);
        self.downstream.on_completed();
    }
}

impl IDisposable for MergedExceptionsSink {
    fn dispose(&self) {
        dispose_all(&self.subscriptions);
    }
}

struct MergedExceptionsElement {
    parent: Rc<MergedExceptionsSink>,
}

impl IObserver<ObservableError> for MergedExceptionsElement {
    fn on_next(&self, value: ObservableError) {
        self.parent.on_next_at(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error_at(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed_at();
    }
}
