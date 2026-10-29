//! The framework-independent docking contracts and engine of FerroDock.
//!
//! Modules follow the source namespaces. See the workspace's porting inventory
//! for the implementation status of each source file.

extern crate self as ferrodock_model;

/// A shared value of any type, compatible with the framework's object value representation.
pub type BoxedValue = std::rc::Rc<dyn std::any::Any>;

/// Plain, custom and observable list references used by the model contracts.
pub mod collections;

/// Source stream contracts and memory streams shared by serializer implementations.
pub mod io;

/// Source fluent creation and configuration helpers on shared model references.
pub mod fluent_extensions;

mod factory_base;
pub use factory_base::{DockableQuery, DockableQueryEnumerator, FactoryBase, FactoryBaseImpl};

mod dock_capability_resolver;
pub use dock_capability_resolver::DockCapabilityResolver;

mod dock_service;
pub use dock_service::DockService;
mod dock_manager;
pub use dock_manager::DockManager;
mod dock_outer_split;
mod dock_state;
pub use dock_state::DockState;
mod dock_workspace;
pub use dock_workspace::DockWorkspace;
mod dock_workspace_tracking_options;
pub use dock_workspace_tracking_options::{DockWorkspaceTrackingOptions, WorkspaceDockableFilter};
mod dock_workspace_dirty_changed_event_args;
pub use dock_workspace_dirty_changed_event_args::DockWorkspaceDirtyChangedEventArgs;
mod dock_workspace_manager;
mod workspace_ordinal_data;
pub use dock_workspace_manager::{DockWorkspaceManager, WorkspaceCollection, WorkspaceIterator};

/// Adapters for model tracking, navigation, and window hosting.
pub mod adapters;
/// Definitions and providers of dock command bars.
pub mod command_bars;
/// Dock, document, tool, and layout container contracts.
pub mod controls;
/// Docking contracts, policies, geometry, and operation types.
pub mod core;
/// Framework-independent application service contracts.
pub mod services;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod dock_capability_resolver_tests;

/// Native untyped value support shared by the source model implementations.
#[doc(hidden)]
pub mod value_semantics;

mod native_reference;
/// Source-runtime string comparisons shared by ported units.
#[doc(hidden)]
pub mod source_string;
