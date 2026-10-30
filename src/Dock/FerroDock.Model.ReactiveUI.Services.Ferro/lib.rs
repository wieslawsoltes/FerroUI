//! FerroUI integration for the ReactiveUI docking services.
/// Reactive view classes and their typed handles.
pub mod controls;

mod register_types;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod activation_tests;
