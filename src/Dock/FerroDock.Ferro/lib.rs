//! Native FerroUI controls for the source FerroDock docking engine.
pub mod controls;
mod register_types;
pub use register_types::{register_types, ASSEMBLY};

mod contract_metadata;
#[doc(hidden)]
pub use contract_metadata::{
    ConfirmationRequestCollectionMetadata, DialogRequestCollectionMetadata, ICommandMetadata,
    IDockBusyServiceMetadata, IDockConfirmationServiceMetadata, IDockDialogServiceMetadata,
    IDockGlobalBusyServiceMetadata, IDockGlobalConfirmationServiceMetadata,
    IDockGlobalDialogServiceMetadata, IVisualTreeLifecycleMetadata,
};

mod request_items;

mod request_metadata;
#[doc(hidden)]
pub use request_metadata::{ConfirmationRequestMetadata, DialogRequestMetadata};

#[doc(hidden)]
pub use contract_metadata::IHostOverlayServicesMetadata;

#[doc(hidden)]
pub use ferrodock_settings::DockOperationMetadata;

#[cfg(test)]
mod enum_metadata_tests;

pub mod behaviors;

mod automation;
pub mod model_projection;

mod internal;
pub mod mutable_items_list;
#[cfg(test)]
mod root_control_tests;

pub mod contract;

#[cfg(test)]
mod dock_target_tests;

pub mod mdi;

/// Reexports needed by original managed model composition macros.
#[doc(hidden)]
pub mod __private {
    pub use ferrodock_controls_recycling::IControlRecyclingIdProvider;
    pub use ferrodock_model::{core::*, BoxedValue};
    pub use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
}

#[doc(hidden)]
pub use ferrodock_model_ferro::IDockItemContainerMetadataDescriptor;

#[cfg(test)]
mod drag_preview_tests;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDockableMetadata;

/// Original docking value converters.
pub mod converters;

#[cfg(test)]
mod converter_tests;

#[cfg(test)]
mod selector_tests;
pub mod selectors;

pub mod command_bars;
mod read_only_items;

mod model_items;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDockMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDocumentDockMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::IToolDockMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDockCapabilityPolicyMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::DockCapabilityPolicyMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::DockCapabilityOverridesMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::DockableListMetadata;

mod dock_model_registration;

#[cfg(test)]
mod dock_model_metadata_tests;

#[cfg(test)]
mod capability_converter_tests;

#[doc(hidden)]
pub use ferrodock_model_ferro::IRootDockMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::{FactoryExtensionsMetadata, IFactoryMetadata};

/// Original docking diagnostic events.
pub mod diagnostics;

mod services;

#[cfg(test)]
mod host_control_integration_tests;

/// Original native UI commands.
pub mod commands;

#[cfg(test)]
mod theme_converter_tests;

mod dock_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IDockDockMetadata;

mod proportional_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IProportionalDockMetadata;

mod grid_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IGridDockMetadata;

mod stack_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IStackDockMetadata;

mod wrap_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IWrapDockMetadata;

mod uniform_grid_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IUniformGridDockMetadata;

mod split_view_dock_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::ISplitViewDockMetadata;

mod document_content_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IDocumentContentMetadata;

mod tool_content_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IToolContentMetadata;

mod global_target_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IGlobalTargetMetadata;

#[cfg(test)]
mod layout_converter_tests;

#[cfg(test)]
mod content_contract_metadata_tests;

/// Original docking theme configuration contracts.
pub mod themes;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDockWindowMetadata;

mod splitter_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::ISplitterMetadata;

mod proportional_dock_splitter_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IProportionalDockSplitterMetadata;

mod grid_dock_splitter_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IGridDockSplitterMetadata;

#[doc(hidden)]
pub use ferrodock_model_ferro::IDocumentTemplateMetadata;

mod document_dock_content_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IDocumentDockContentMetadata;

mod tool_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IToolMetadata;

mod document_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::IDocumentMetadata;

mod local_target_metadata;
#[doc(hidden)]
pub use ferrodock_model_ferro::ILocalTargetMetadata;
