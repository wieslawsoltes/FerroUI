//! Reusable native workspace for editing FerroUI XAML.
//!
//! [`DesignerSession`] is the UI-thread transaction boundary used by source editing,
//! visual tools and automation. [`DesignerWorkspace`] is an optional embeddable view
//! of that session. Neither starts a window, server or background transport.
mod collaboration;
mod editors;
mod preview_source;
mod runtime;
mod session;
mod surface;
mod timeline_lane;
mod workspace;

pub use editors::{PropertyEditorContext, PropertyEditorFactory};
pub use ferroui_designer_core::{Document, Edit, NodeId};
pub use ferroui_designer_tools as tools;
pub use runtime::{PreviewStatus, RuntimeNode};
pub use session::DesignerSession;
pub use surface::{DesignGesture, DesignSurface};
pub use workspace::{DesignerWorkspace, OutlineMode};

/// The self-contained example opened by Designer Studio and usable by hosts.
pub const STARTER_XAML: &str = include_str!("Starter.xaml");

#[cfg(test)]
mod tests;
mod workspace_tools;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(all(test, feature = "render-smoke"))]
mod render_tests;
