#![recursion_limit = "512"]
//! Native metadata for the original UI-independent docking model assembly.
mod enum_text;
mod register_types;
mod foreign_enums {
    include!(concat!(env!("OUT_DIR"), "/foreign_enums.rs"));
}
#[doc(hidden)]
pub use foreign_enums::{
    AlignmentMetadata, DockCapabilityMetadata, DockCapabilityValueSourceMetadata,
    DockCommandBarKindMetadata, DockCommandBarMergeModeMetadata,
    DockFloatingWindowHostModeMetadata, DockModeMetadata, DockOperationMetadata,
    DockTrackingChangeReasonMetadata, DockWindowOwnerModeMetadata, DockWindowStateMetadata,
    DockingWindowStateMetadata, DocumentCloseButtonShowModeMetadata, DocumentLayoutModeMetadata,
    DocumentTabLayoutMetadata, GridResizeDirectionMetadata, GripModeMetadata,
    MdiWindowStateMetadata, OrientationMetadata, PinnedDockDisplayModeMetadata,
    SplitViewDisplayModeMetadata, SplitViewPanePlacementMetadata, TrackingModeMetadata,
};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod enum_metadata_tests;

mod factory_base_reference;
pub use factory_base_reference::FactoryBaseReference;
mod factory_registry_reference;
pub use factory_registry_reference::{
    DockControlListMetadata, DockableControlMapReference, DockableObjectMapReference,
    HostWindowListMetadata,
};

#[doc(hidden)]
pub mod event_metadata;
#[doc(hidden)]
pub mod factory_events;

#[cfg(test)]
mod factory_events_tests;

mod factory_metadata;
#[doc(hidden)]
pub use factory_metadata::{FactoryExtensionsMetadata, IFactoryMetadata};
mod model_handles;

#[cfg(test)]
mod dock_metadata_tests;

mod factory_locators;
pub use factory_locators::{
    ContextFactoryReference, ContextLocatorMapReference, DockableFactoryReference,
    DockableLocatorMapReference, HostWindowFactoryReference, HostWindowLocatorMapReference,
};

#[cfg(test)]
mod factory_locators_tests;

mod factory_search;
pub use factory_search::{
    DockablePredicate, DockablePredicateReference, DockableQueryEnumeratorReference,
    DockableQueryReference, RootPredicateReference,
};
#[cfg(test)]
mod factory_operations_tests;

#[cfg(test)]
mod factory_dispatch_tests;

mod capability_policy_metadata;
#[doc(hidden)]
pub use capability_policy_metadata::IDockCapabilityPolicyMetadata;

mod dock_capability_policy_metadata;
#[doc(hidden)]
pub use dock_capability_policy_metadata::DockCapabilityPolicyMetadata;

mod dock_capability_overrides_metadata;
#[doc(hidden)]
pub use dock_capability_overrides_metadata::DockCapabilityOverridesMetadata;

mod dock_window_metadata;
#[doc(hidden)]
pub use dock_window_metadata::IDockWindowMetadata;

#[cfg(test)]
mod dock_window_metadata_tests;

mod capability_registration;
mod core_options_metadata;
#[doc(hidden)]
pub use core_options_metadata::{
    DockCapabilityEvaluationMetadata, DockCapabilityResolverMetadata, DockWindowOptionsMetadata,
};

#[cfg(test)]
mod core_options_tests;

mod dockable_metadata;
#[doc(hidden)]
pub use dockable_metadata::IDockableMetadata;

mod dock_metadata;
#[doc(hidden)]
pub use dock_metadata::IDockMetadata;

mod dockable_list_metadata;
#[doc(hidden)]
pub use dockable_list_metadata::DockableListMetadata;

#[cfg(test)]
mod dockable_metadata_tests;

mod local_target_metadata;
#[doc(hidden)]
pub use local_target_metadata::ILocalTargetMetadata;
mod global_target_metadata;
#[doc(hidden)]
pub use global_target_metadata::IGlobalTargetMetadata;
mod container_metadata;
#[doc(hidden)]
pub use container_metadata::IDockItemContainerMetadataDescriptor;
#[cfg(test)]
mod core_marker_metadata_tests;
mod host_contract_metadata;
#[cfg(test)]
mod host_contract_metadata_tests;
#[doc(hidden)]
pub use host_contract_metadata::{
    IDockManagerStateMetadata, IHostWindowMetadata, IHostWindowStateMetadata,
};

mod dock_dock_metadata;
#[doc(hidden)]
pub use dock_dock_metadata::IDockDockMetadata;
mod document_content_metadata;
#[doc(hidden)]
pub use document_content_metadata::IDocumentContentMetadata;
mod document_dock_content_metadata;
#[doc(hidden)]
pub use document_dock_content_metadata::IDocumentDockContentMetadata;
mod document_dock_metadata;
#[doc(hidden)]
pub use document_dock_metadata::IDocumentDockMetadata;
mod document_metadata;
#[doc(hidden)]
pub use document_metadata::IDocumentMetadata;
mod document_template_metadata;
#[doc(hidden)]
pub use document_template_metadata::IDocumentTemplateMetadata;
mod grid_dock_metadata;
#[doc(hidden)]
pub use grid_dock_metadata::IGridDockMetadata;
mod grid_dock_splitter_metadata;
#[doc(hidden)]
pub use grid_dock_splitter_metadata::IGridDockSplitterMetadata;
mod proportional_dock_metadata;
#[doc(hidden)]
pub use proportional_dock_metadata::IProportionalDockMetadata;
mod proportional_dock_splitter_metadata;
#[doc(hidden)]
pub use proportional_dock_splitter_metadata::IProportionalDockSplitterMetadata;
mod root_dock_metadata;
#[doc(hidden)]
pub use root_dock_metadata::IRootDockMetadata;
mod split_view_dock_metadata;
#[doc(hidden)]
pub use split_view_dock_metadata::ISplitViewDockMetadata;
mod splitter_metadata;
#[doc(hidden)]
pub use splitter_metadata::ISplitterMetadata;
mod stack_dock_metadata;
#[doc(hidden)]
pub use stack_dock_metadata::IStackDockMetadata;
mod tool_content_metadata;
#[doc(hidden)]
pub use tool_content_metadata::IToolContentMetadata;
mod tool_dock_metadata;
#[doc(hidden)]
pub use tool_dock_metadata::IToolDockMetadata;
mod tool_metadata;
#[doc(hidden)]
pub use tool_metadata::IToolMetadata;
mod uniform_grid_dock_metadata;
#[doc(hidden)]
pub use uniform_grid_dock_metadata::IUniformGridDockMetadata;
mod wrap_dock_metadata;
#[doc(hidden)]
pub use wrap_dock_metadata::IWrapDockMetadata;
mod mdi_document_metadata;
#[doc(hidden)]
pub use mdi_document_metadata::IMdiDocumentMetadata;
mod control_contract_registration;

mod tool_template_metadata;
#[doc(hidden)]
pub use tool_template_metadata::IToolTemplateMetadata;
mod tool_dock_content_metadata;
#[doc(hidden)]
pub use tool_dock_content_metadata::IToolDockContentMetadata;
mod dock_window_list_metadata;
#[doc(hidden)]
pub use dock_window_list_metadata::DockWindowListMetadata;
#[cfg(test)]
mod control_contract_metadata_tests;

mod document_factory_reference;
pub use document_factory_reference::DocumentFactoryReference;
mod document_dock_factory_metadata;
#[doc(hidden)]
pub use document_dock_factory_metadata::IDocumentDockFactoryMetadata;
#[cfg(test)]
mod document_factory_metadata_tests;

mod dock_window_host_factory_metadata;
#[doc(hidden)]
pub use dock_window_host_factory_metadata::IDockWindowHostFactoryMetadata;
