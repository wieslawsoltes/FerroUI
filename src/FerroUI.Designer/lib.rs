//! Reusable native workspace for editing FerroUI XAML.
//!
//! [`DesignerSession`] is the UI-thread transaction boundary used by source editing,
//! visual tools and automation. [`DesignerWorkspace`] is an optional embeddable view
//! of that session. Neither starts a window, server or background transport.
mod affine_manipulation;
mod asset_toolbox;
mod animation_session;
mod animation_runtime;
mod collaboration;
pub mod sharing;
mod editors;
mod interaction_session;
mod preview_host;
mod preview_source;
mod runtime;
mod session;
mod surface;
mod timeline_lane;
mod workspace;

pub use editors::{PropertyEditorContext, PropertyEditorFactory};
pub use asset_toolbox::{AssetIcon, AssetToolbox, DesignerAsset, default_designer_assets};
pub use ferroui_designer_core::{Document, Edit, NodeId};
pub use ferroui_designer_tools as tools;
pub use preview_host::{
    PreviewFactory, PreviewHostOptions, PreviewRequest, ResourcePreviewFactory,
};
pub use runtime::{PreviewStatus, RuntimeNode};
pub use session::DesignerSession;
pub use surface::{DesignGesture, DesignSurface};
pub use timeline_lane::TimelineLane;
pub use workspace::{DesignerWorkspace, OutlineMode};
pub use workspace::shell::{NavigationPane, WorkspaceViewMode};

/// The self-contained example opened by Designer Studio and usable by hosts.
pub const STARTER_XAML: &str = include_str!("Starter.xaml");

#[cfg(test)]
mod native_extensions_tests;
#[cfg(test)]
mod workspace_authoring_tests;
#[cfg(test)]
mod workspace_shell_tests;
#[cfg(test)]
mod tests;
mod workspace_tools;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(all(test, feature = "render-smoke"))]
mod render_tests;

#[cfg(test)]
mod animation_integration_tests;
