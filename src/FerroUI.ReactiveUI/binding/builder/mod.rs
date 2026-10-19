//! `ReactiveUI.Binding/Builder`: the registrations of the services of the property observation.
//!
//! The builder itself (`ReactiveUIBindingBuilder`, `RxBindingBuilder`) is not in this module yet.

mod reactive_ui_binding_module;

pub use reactive_ui_binding_module::{register_observation_defaults, ReactiveUIBindingModule};
