//! `ReactiveCommandBase<TParam, TResult>` (`ReactiveUI.Shared/ReactiveCommand/ReactiveCommandBase.cs`):
//! what every reactive command shares, the `ICommand` side of it.
//!
//! Upstream it is the abstract base class of [`ReactiveCommand`](super::ReactiveCommand) and
//! [`CombinedReactiveCommand`](super::CombinedReactiveCommand), whose abstract members are the
//! members of [`IReactiveCommandOf`](super::IReactiveCommandOf). Rust has no base classes: the
//! state it holds (the `CanExecuteChanged` handlers and the value `ICommand.CanExecute` answers) is
//! [`ReactiveCommandBase`], which each command holds, and the `ICommand.Execute` it implements is
//! [`icommand_execute`].

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::Observer;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The `ICommand` state of a reactive command (`ReactiveCommandBase<TParam, TResult>`): the
/// handlers of `CanExecuteChanged` and the value `ICommand.CanExecute` answers.
#[derive(Default)]
pub(crate) struct ReactiveCommandBase {
    /// `_canExecuteChanged`.
    can_execute_changed: Event<()>,
    /// `_canExecuteValue`.
    can_execute_value: Cell<bool>,
}

impl ReactiveCommandBase {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Records the new value of `CanExecute` and raises `CanExecuteChanged`
    /// (`OnCanExecuteChanged(bool)`).
    pub(crate) fn on_can_execute_changed(&self, new_value: bool) {
        self.can_execute_value.set(new_value);
        self.can_execute_changed.raise(&());
    }

    /// The answer of `ICommand.CanExecute`, whatever the parameter (`ICommandCanExecute(object?)`).
    pub(crate) fn icommand_can_execute(&self, _parameter: Option<&dyn Any>) -> bool {
        self.can_execute_value.get()
    }

    /// Adds a handler of `CanExecuteChanged`; disposing the result removes it
    /// (`ICommand.CanExecuteChanged` add and remove).
    pub(crate) fn add_can_execute_changed(self: &Rc<Self>, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(Rc::new(move |_: &()| handler()));
        let base: Weak<Self> = Rc::downgrade(self);
        Disposable::create(move || {
            if let Some(base) = base.upgrade() {
                base.can_execute_changed.remove(token);
            }
        })
    }
}

/// `ICommand.Execute` of a reactive command (`ICommandExecute(object?)`): a missing parameter is
/// the default of `P`, a parameter of another type than `P` panics with an
/// `InvalidOperationException` message, and the execution runs to its end with its error ignored
/// (the error goes to `ThrownExceptions`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the message names the type the command requires; the
/// type of the parameter it received is not known (`dyn Any` has no type name).
pub(crate) fn icommand_execute<P: Clone + Default + 'static, R: 'static>(
    parameter: Option<&dyn Any>,
    execute: impl FnOnce(P) -> Rc<dyn IObservable<R>>,
) {
    let parameter = match parameter {
        None => P::default(),
        Some(value) => match value.downcast_ref::<P>() {
            Some(value) => value.clone(),
            None => panic!(
                "InvalidOperationException: Command requires parameters of type {}, but received parameter of another type.",
                std::any::type_name::<P>()
            ),
        },
    };
    let result = execute(parameter);
    // Fire-and-forget: drive the execution to completion and swallow any error here (errors are
    // surfaced through ThrownExceptions).
    let _ = result.subscribe(Observer::create_all(|_| {}, |_| {}, || {}));
}
