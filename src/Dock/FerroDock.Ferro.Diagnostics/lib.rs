//! Original diagnostic controls and presentation helpers for FerroDock.
/// Original diagnostic inspector controls.
pub mod controls;
mod helpers;
mod register_types;
pub use register_types::{register_types, ASSEMBLY};
mod dock_debug_overlay_extensions;
mod dock_debug_overlay_manager;
pub use dock_debug_overlay_extensions::DockDebugOverlayExtensions;

#[cfg(test)]
mod debug_overlay_tests;
#[cfg(test)]
mod selection_overlay_tests;

extern crate self as ferrodock_ferro_diagnostics;
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{
    compiled_dock_debug_view, compiled_dock_dock_debug_view, compiled_dock_window_debug_view,
    compiled_dockable_debug_view, compiled_document_content_debug_view,
    compiled_document_dock_content_debug_view, compiled_document_dock_debug_view,
    compiled_document_template_debug_view, compiled_grid_dock_debug_view,
    compiled_grid_dock_splitter_debug_view, compiled_markup, compiled_proportional_dock_debug_view,
    compiled_proportional_dock_splitter_debug_view, compiled_root_dock_debug,
    compiled_root_dock_info_debug_view, compiled_stack_dock_debug_view,
    compiled_tool_content_debug_view, compiled_tool_dock_debug_view,
    compiled_uniform_grid_dock_debug_view, compiled_wrap_dock_debug_view,
};

#[cfg(test)]
mod inspector_view_tests;

/// Original inspector behaviors extracted from view code.
pub mod behaviors;
mod infrastructure;
/// Framework-independent inspector presentation and its clock contract.
pub mod view_models;

#[cfg(test)]
mod root_dock_debug_tests;
