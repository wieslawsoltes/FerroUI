//! Reusable native workspace for editing FerroUI XAML.
//!
//! [`DesignerSession`] is the UI-thread transaction boundary used by source editing,
//! visual tools and automation. [`DesignerWorkspace`] is an optional embeddable view
//! of that session. Neither starts a window, server or background transport.
mod affine_manipulation;
mod animation_runtime;
mod animation_session;
mod asset_toolbox;
mod chrome;
mod collaboration;
mod editors;
mod grid_track_editor;
mod interaction_session;
mod layout_helper_renderer;
mod preview_host;
mod preview_source;
mod property_color_editor;
mod property_controls;
mod runtime;
mod session;
pub mod sharing;
mod surface;
mod timeline_lane;
mod workspace;

pub use asset_toolbox::{
    AssetIcon, AssetToolbox, AssetViewMode, DesignerAsset, default_designer_assets,
};
pub use chrome::designer_chrome_style;
pub use editors::{PropertyEditorContext, PropertyEditorFactory};
pub use ferroui_designer_core::{Document, Edit, NodeId};
pub use ferroui_designer_tools as tools;
pub use grid_track_editor::GridTrackEditor;
pub use preview_host::{
    PreviewFactory, PreviewHostOptions, PreviewRequest, ResourcePreviewFactory,
};
pub use property_color_editor::PropertyColorEditor;
pub use property_controls::{
    PropertyComponentsEditor, PropertyComponentsKind, PropertyNumberOptions,
    PropertyNumberScrubber, property_value_data_format,
};
pub use runtime::{PreviewStatus, RuntimeNode};
pub use session::DesignerSession;
pub use surface::{DesignGesture, DesignSurface};
pub use timeline_lane::{EasingPreview, TimelineLane};
pub use workspace::shell::{NavigationPane, WorkspaceViewMode};
pub use workspace::{DesignerWorkspace, OutlineCommand, OutlineMode};

/// The self-contained example opened by Designer Studio and usable by hosts.
pub const STARTER_XAML: &str = include_str!("Starter.xaml");

#[cfg(test)]
mod native_extensions_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod workspace_authoring_tests;
mod workspace_layout_helpers;
#[cfg(test)]
mod workspace_shell_tests;
mod workspace_tools;

pub use layout_helper_renderer::{
    DragVisualGhost, LayoutHelperRenderState, hit_test_layout_handle, hover_layout_handle,
    render_layout_helpers, render_layout_helpers_with_state, render_manipulation_metrics,
};
pub use workspace_layout_helpers::{LayoutEditPreview, LayoutHelperSnapshot};

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(all(test, feature = "render-smoke"))]
mod render_tests;

#[cfg(test)]
mod animation_integration_tests;
