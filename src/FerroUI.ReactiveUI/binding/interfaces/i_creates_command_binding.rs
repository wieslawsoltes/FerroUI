//! `ICreatesCommandBinding` (ReactiveUI.Binding, `Interfaces/ICreatesCommandBinding.cs`).

use crate::reactive_command::ICommand;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// A command parameter (`object?`): the value the command is executed with, of any type, or none.
/// A command receives it as `Option<&dyn Any>` (`parameter.as_deref()`).
pub type CommandParameter = Option<Rc<dyn Any>>;

/// Adds a handler to an event of a control and returns the disposable that removes it: the
/// counterpart of the pair `addHandler` / `removeHandler` of `Action<EventHandler<TEventArgs>>`.
pub type CommandEventSubscription = Rc<dyn Fn(Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// A plugin that binds an [`ICommand`] to a control (`ICreatesCommandBinding`). The
/// implementations are registered in the service locator (Splat) as `dyn ICreatesCommandBinding`,
/// and the one with the highest affinity for the type of a control binds it
/// ([`CommandBinderService`](crate::CommandBinderService)), as upstream.
///
/// The crate registers none: [`CreatesCommandBindingViaEvent`](crate::CreatesCommandBindingViaEvent)
/// and [`CreatesCommandBindingViaCommandParameter`](crate::CreatesCommandBindingViaCommandParameter)
/// bind the controls of one type, and a platform registers its own (the controls of FerroUI).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the trait is object safe. The type parameter `T` of the
/// control is a `TypeId` in [`get_affinity_for_object`](Self::get_affinity_for_object) and the
/// control an `Rc<dyn Any>` in the bindings; `TEventArgs` has no counterpart (the handler ignores
/// the arguments of the event); the add and remove handlers are one
/// [`CommandEventSubscription`]; the command parameter is an observable of [`CommandParameter`].
pub trait ICreatesCommandBinding {
    /// A positive number when this plugin can bind a command to an object of the type
    /// `target_type`, the highest number winning among the plugins; zero or less when it cannot
    /// (`GetAffinityForObject<T>(bool hasEventTarget)`). `has_event_target` says whether the caller
    /// names the event to bind to.
    fn get_affinity_for_object(&self, target_type: TypeId, has_event_target: bool) -> i32;

    /// Binds `command` to `target` through the default event of the target (`Click`, ...), executed
    /// with the latest value of `command_parameter`; `None` when no binding was made, as for a
    /// missing command (`BindCommandToObject<T>`). Disposing the result unbinds the command.
    fn bind_command_to_object(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>>;

    /// Binds `command` to the event named `event_name` of `target`
    /// (`BindCommandToObject<T, TEventArgs>(..., string eventName)`).
    fn bind_command_to_object_event(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        event_name: &str,
    ) -> Option<Rc<dyn IDisposable>>;

    /// Binds `command` to the event that `subscription` adds a handler to
    /// (`BindCommandToObject<T, TEventArgs>(..., addHandler, removeHandler)`).
    fn bind_command_to_object_with_handlers(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>>;
}
