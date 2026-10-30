//! ReactiveUI docking models, following the source's model hierarchy through composition.

/// Concrete document, tool, splitter, and dock models.
pub mod controls;
/// Reactive model base components and their virtual contracts.
pub mod core;

mod factory;
pub use factory::Factory;
mod factory_impl;
pub use factory_impl::FactoryImpl;

/// Types used by the inherited implementation macro.
#[doc(hidden)]
pub mod __private {
    pub use ferrodock_controls_recycling_model::IControlRecyclingIdProvider;
    pub use ferrodock_model::collections::ListHandle;
    pub use ferrodock_model::controls::IRootDock;
    pub use ferrodock_model::controls::*;
    pub use ferrodock_model::core::events::*;
    pub use ferrodock_model::core::*;
    pub use ferrodock_model::BoxedValue;
    pub use ferrodock_model::{DockableQuery, FactoryBaseImpl};
    pub use ferroui_reactiveui::{ICommand, ReactiveObject};
    pub use ferroui_system::component_model::INotifyPropertyChanged;
    pub use ferroui_system::IDisposable;
    pub use std::rc::Rc;
}

#[cfg(test)]
mod dock_tests;
#[cfg(test)]
mod document_dock_tests;
#[cfg(test)]
#[path = "../FerroDock.Model/core/factory_parameter_tests.rs"]
mod factory_parameter_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
#[path = "../FerroDock.Model/test_support.rs"]
#[allow(dead_code)] // This suite reuses a subset of the shared contract fixtures.
mod test_support;

#[cfg(test)]
mod dock_property_tests;

#[cfg(test)]
mod inheritance_tests;
#[cfg(test)]
mod test_host;
#[cfg(test)]
mod window_tests;

#[cfg(test)]
mod window_property_tests;

#[cfg(test)]
mod factory_state_tests;

#[cfg(test)]
mod factory_init_tests;

#[cfg(test)]
mod factory_tracking_tests;

#[cfg(test)]
mod factory_focus_tests;

#[cfg(test)]
mod factory_event_tests;

#[cfg(test)]
mod factory_window_tests;

#[cfg(test)]
mod factory_mutation_tests;

#[cfg(test)]
mod factory_pin_tests;

#[cfg(test)]
mod factory_remove_tests;

#[cfg(test)]
mod factory_split_tests;

#[cfg(test)]
mod factory_float_tests;

#[cfg(test)]
mod factory_metadata_tests;

#[cfg(test)]
mod factory_close_tests;

#[cfg(test)]
mod production_factory_tests;

#[cfg(test)]
mod dock_manager_tests;
#[cfg(test)]
mod dock_outer_split_tests;
#[cfg(test)]
mod dock_service_tests;
#[cfg(test)]
mod dock_state_tests;
#[cfg(test)]
mod engine_test_support;

#[cfg(test)]
mod dock_workspace_tests;

#[cfg(test)]
mod fluent_tests;

/// UI-independent service contracts for reactive docking applications.
pub mod services;

#[cfg(test)]
mod target_marker_tests;

#[cfg(test)]
mod factory_extensions_tests;

#[cfg(test)]
mod nullable_factory_tests;

#[cfg(test)]
mod nullable_state_tests;

#[cfg(test)]
mod nullable_mutation_tests;

#[cfg(test)]
mod nullable_target_tests;

#[cfg(test)]
mod nullable_move_tests;

#[cfg(test)]
mod nullable_preview_tests;

#[cfg(test)]
mod nullable_hidden_tests;

#[cfg(test)]
mod nullable_floating_tests;

#[cfg(test)]
mod nullable_focus_tests;

#[cfg(test)]
mod nullable_window_options_tests;

#[cfg(test)]
mod nullable_windows_tests;

#[cfg(test)]
mod nullable_search_tests;

#[cfg(test)]
mod query_cursor_tests;

#[cfg(test)]
mod serialized_reference_tests;
