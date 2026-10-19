//! ReactiveUI.Binding (the run time): property observation, output properties and, in later
//! waves, the bindings of views (docs/porting/reactive.md, section 3.2).
//!
//! The modules follow the folders of `ReactiveUI.Binding.Shared`. Upstream's source generator
//! reads each lambda at build time; here a [`PropertyPath`](ferroui_system::component_model::PropertyPath)
//! (`prop!(VM => a.b)`) carries the getter and the names of the steps, and the observation goes
//! through [`ICreatesObservableForProperty`](interfaces::ICreatesObservableForProperty), step by
//! step, as upstream's run-time fallback does.

pub mod bindings;
pub mod builder;
pub mod command_binding;
pub mod interactions;
pub mod interfaces;
pub mod mixins;
pub mod observable_for_property;
pub mod observables;
pub mod view;
