//! `ReactiveUIBindingExtensions.InvokeCommand` (ReactiveUI.Binding,
//! `Mixins/ReactiveUIBindingExtensions.InvokeCommand.cs`): execute a command with each value an
//! observable produces.
//!
//! The command is offered every value as its parameter, and a value it cannot execute is dropped.
//! A reactive command is reached through these methods like any other [`ICommand`]: the parameter
//! is passed as `&dyn Any`, which the command reads as its parameter type.

use super::ReactiveUIBindingExtensions;
use crate::binding::command_binding::CommandInvoker;
use crate::reactive_command::ICommand;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// The `InvokeCommand` methods of `ReactiveUIBindingExtensions`, on an observable.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the overload that names the property holding the command
/// takes a [`PropertyPath`] (`prop!(VM => command)`) and is
/// [`invoke_command_path`](Self::invoke_command_path); upstream's generator writes its dispatch
/// from the lambda, and its `InvokeCommandUnsafe` reads the expression by reflection. An error of
/// the values is rethrown where it is raised: here it panics. The null checks of the arguments
/// have no counterpart.
pub trait InvokeCommandExt<T> {
    /// Executes `command` with each value of the sequence that it can execute
    /// (`InvokeCommand<T>(IObservable<T>, ICommand)`); disposing the result stops it.
    fn invoke_command(&self, command: impl ICommand + 'static) -> Rc<dyn IDisposable>;

    /// Executes the command that `command_property` of `target` holds with each value of the
    /// sequence, skipping a value while there is no command or it cannot execute the value; the
    /// property is observed, so a command set later is used from then on
    /// (`InvokeCommand<T, TTarget>(IObservable<T>, TTarget?, Expression<Func<TTarget, ICommand?>>)`).
    /// Without a target nothing is subscribed. Disposing the result stops the executions and the
    /// observation.
    fn invoke_command_path<V: 'static, C: ICommand + Clone + PartialEq + 'static>(
        &self,
        target: Option<Rc<V>>,
        command_property: PropertyPath<Rc<V>, Option<C>>,
    ) -> Rc<dyn IDisposable>;
}

impl<T: 'static> InvokeCommandExt<T> for Rc<dyn IObservable<T>> {
    fn invoke_command(&self, command: impl ICommand + 'static) -> Rc<dyn IDisposable> {
        CommandInvoker::invoke(self, Rc::new(command))
    }

    fn invoke_command_path<V: 'static, C: ICommand + Clone + PartialEq + 'static>(
        &self,
        target: Option<Rc<V>>,
        command_property: PropertyPath<Rc<V>, Option<C>>,
    ) -> Rc<dyn IDisposable> {
        let Some(target) = target else {
            return Disposable::empty();
        };
        let commands: Rc<dyn IObservable<Option<Rc<dyn ICommand>>>> = target.when_any_value(command_property).select(|command: Option<C>| {
            command.map(|command| {
                let command: Rc<dyn ICommand> = Rc::new(command);
                command
            })
        });
        CommandInvoker::invoke_observed(self, &commands)
    }
}
