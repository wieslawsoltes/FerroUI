//! `CommandBinding` (ReactiveUI.Binding): the execution of commands from observables.
//!
//! `CommandBinderService` (the binding of commands to the controls of a view) belongs to a later
//! wave.

mod command_invoker;

pub use command_invoker::CommandInvoker;
