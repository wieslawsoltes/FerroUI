//! `CommandInvoker` (ReactiveUI.Binding, `CommandBinding/CommandInvoker.cs`): executes a command
//! with each value of an observable, which is what `InvokeCommand` runs.

use crate::reactive_command::ICommand;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Executes a command with each value of an observable as its parameter, skipping a value the
/// command cannot execute (`CommandInvoker`).
///
/// An error of the values, or of the observable of commands, is rethrown where it is raised
/// (`ExceptionDispatchInfo.Throw`): here it panics with its message.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the overload on an observable of commands is
/// [`invoke_observed`](Self::invoke_observed); the null checks of the arguments have no
/// counterpart.
pub struct CommandInvoker;

impl CommandInvoker {
    /// Executes `command` with each value of `source` that it can execute
    /// (`Invoke<T>(IObservable<T>, ICommand)`); disposing the result stops it.
    pub fn invoke<T: 'static>(source: &Rc<dyn IObservable<T>>, command: Rc<dyn ICommand>) -> Rc<dyn IDisposable> {
        source.subscribe(Rc::new(FixedCommandObserver { command }))
    }

    /// Executes the latest command of `commands` with each value of `source` that it can execute,
    /// and nothing while there is none (`Invoke<T>(IObservable<T>, IObservable<ICommand?>)`);
    /// disposing the result ends both subscriptions.
    pub fn invoke_observed<T: 'static>(
        source: &Rc<dyn IObservable<T>>,
        commands: &Rc<dyn IObservable<Option<Rc<dyn ICommand>>>>,
    ) -> Rc<dyn IDisposable> {
        let latch = Rc::new(CommandLatch { command: RefCell::new(None) });
        let command_subscription = commands.subscribe(latch.clone());
        let source_subscription = source.subscribe(Rc::new(LatchedCommandObserver { latch }));
        CompositeDisposable::from_disposables([command_subscription, source_subscription])
    }
}

/// Rethrows an error where it was raised (`ExceptionDispatchInfo.Capture(error).Throw()`).
fn rethrow(error: ObservableError) -> ! {
    panic!("{error}")
}

/// `FixedCommandObserver<T>`.
struct FixedCommandObserver {
    command: Rc<dyn ICommand>,
}

impl<T: 'static> IObserver<T> for FixedCommandObserver {
    fn on_next(&self, value: T) {
        let parameter: &dyn Any = &value;
        if !self.command.can_execute(Some(parameter)) {
            return;
        }
        self.command.execute(Some(parameter));
    }

    fn on_error(&self, error: ObservableError) {
        rethrow(error);
    }

    fn on_completed(&self) {}
}

/// `CommandLatch`: the latest command of the observable of commands.
struct CommandLatch {
    command: RefCell<Option<Rc<dyn ICommand>>>,
}

impl IObserver<Option<Rc<dyn ICommand>>> for CommandLatch {
    fn on_next(&self, value: Option<Rc<dyn ICommand>>) {
        *self.command.borrow_mut() = value;
    }

    fn on_error(&self, error: ObservableError) {
        rethrow(error);
    }

    fn on_completed(&self) {}
}

/// `LatchedCommandObserver<T>`.
struct LatchedCommandObserver {
    latch: Rc<CommandLatch>,
}

impl<T: 'static> IObserver<T> for LatchedCommandObserver {
    fn on_next(&self, value: T) {
        let Some(command) = self.latch.command.borrow().clone() else { return };
        let parameter: &dyn Any = &value;
        if !command.can_execute(Some(parameter)) {
            return;
        }
        command.execute(Some(parameter));
    }

    fn on_error(&self, error: ObservableError) {
        rethrow(error);
    }

    fn on_completed(&self) {}
}
