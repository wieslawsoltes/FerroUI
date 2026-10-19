//! `IReactiveCommand` (`ReactiveUI.Core/ReactiveCommand/IReactiveCommand.cs`): the members of a
//! reactive command that do not depend on its parameter and result types.

use crate::IHandleObservableErrors;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// The state of a reactive command, whatever its types (`IReactiveCommand`): whether it is
/// executing, whether it can execute, and (through [`IHandleObservableErrors`]) the errors of its
/// executions. Disposing it ends its subscription to the observable that gates it.
pub trait IReactiveCommand: IDisposable + IHandleObservableErrors {
    /// Whether the command is executing: the current value on subscription, then each change
    /// (`IsExecuting`).
    fn is_executing(&self) -> Rc<dyn IObservable<bool>>;

    /// Whether the command can execute: the current value on subscription, then each change; it
    /// is false while the command executes (`CanExecute`).
    fn can_execute(&self) -> Rc<dyn IObservable<bool>>;
}
