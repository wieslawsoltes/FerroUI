//! Dock settings, attached docking properties and app-builder configuration.
mod app_builder_extensions;
mod dock_properties;
mod dock_settings;
mod dock_settings_options;
mod enums;
mod register_types;
pub use app_builder_extensions::AppBuilderExtensions;
pub use dock_properties::DockProperties;
pub use dock_settings::{DiagnosticsLogHandler, DockSettings};
pub use dock_settings_options::DockSettingsOptions;
pub use enums::*;
pub use register_types::{register_types, ASSEMBLY};
// Existing helper paths remain aliases of the single model-owned descriptors.
#[doc(hidden)]
pub use ferrodock_model_ferro::{
    AlignmentMetadata, DockCapabilityMetadata, DockCommandBarKindMetadata,
    DockCommandBarMergeModeMetadata, DockFloatingWindowHostModeMetadata, DockModeMetadata,
    DockOperationMetadata, DockWindowOwnerModeMetadata, DockWindowStateMetadata,
    DockingWindowStateMetadata, DocumentCloseButtonShowModeMetadata, DocumentLayoutModeMetadata,
    DocumentTabLayoutMetadata, GridResizeDirectionMetadata, GripModeMetadata,
    MdiWindowStateMetadata, OrientationMetadata, PinnedDockDisplayModeMetadata,
    TrackingModeMetadata,
};
mod dock_logger;
#[cfg(test)]
mod enum_metadata_tests;
pub use dock_logger::DockLogger;
#[cfg(test)]
mod builder_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod settings_tests;

#[cfg(test)]
mod test_app_builder;
