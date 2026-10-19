//! `IReactiveCommand<TParam, TResult>` (`ReactiveUI.Core/ReactiveCommand/IReactiveCommand{TParam,TResult}.cs`):
//! a reactive command that takes a parameter of type `P` and produces results of type `R`.

use super::IReactiveCommand;
use ferroui_system::IObservable;
use std::rc::Rc;

/// A reactive command with its parameter and result types (`IReactiveCommand<TParam, TResult>`).
/// As an observable it notifies the results of every execution.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): Rust has one name per trait, so the generic interface,
/// which upstream overloads on the name `IReactiveCommand`, is `IReactiveCommandOf`. The overload
/// `Execute()` is [`execute_default`](Self::execute_default).
pub trait IReactiveCommandOf<P, R>: IObservable<R> + IReactiveCommand {
    /// An observable that executes the command with `parameter` when it is subscribed, and
    /// notifies the results of that execution (`Execute(TParam)`). Nothing runs until it is
    /// subscribed.
    fn execute(&self, parameter: P) -> Rc<dyn IObservable<R>>;

    /// [`execute`](Self::execute) with the default parameter (`Execute()`, which passes
    /// `default(TParam)`).
    fn execute_default(&self) -> Rc<dyn IObservable<R>>;
}
