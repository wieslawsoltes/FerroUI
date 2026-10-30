//! ReactiveUI routing integrated with source docking models and services.
/// Navigation and overlay lookup helpers.
pub mod services;

#[macro_use]
pub(crate) mod model_contracts;
/// Routable document, tool and root models.
pub mod controls;
/// Layout-owning routable view models.
pub mod view_models;

#[cfg(test)]
pub(crate) mod navigation_tests;

/// Abstract routable dock components.
pub mod core;
