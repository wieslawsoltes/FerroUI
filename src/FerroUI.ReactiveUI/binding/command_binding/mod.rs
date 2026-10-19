//! `CommandBinding` (ReactiveUI.Binding): the execution of commands from observables
//! ([`CommandInvoker`]) and the binding of commands to controls ([`CommandBinderService`], with
//! the binders [`CreatesCommandBindingViaEvent`] and [`CreatesCommandBindingViaCommandParameter`]).

mod command_binder_service;
mod command_invoker;
mod creates_command_binding_via_command_parameter;
mod creates_command_binding_via_event;

pub use command_binder_service::CommandBinderService;
pub use command_invoker::CommandInvoker;
pub use creates_command_binding_via_command_parameter::CreatesCommandBindingViaCommandParameter;
pub use creates_command_binding_via_event::{
    bind_command_to_event, control_event, ControlEventSubscription, CreatesCommandBindingViaEvent, DEFAULT_COMMAND_EVENTS,
};
