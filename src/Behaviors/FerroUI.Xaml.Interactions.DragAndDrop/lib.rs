//! ferroui-xaml-interactions-draganddrop
//!
//! Drag and drop behaviors, ported from the `Xaml.Behaviors.Interactions.DragAndDrop` project of
//! Xaml.Behaviors (see `NOTICE.md`):
//!
//! - drag sources that start a drag and drop of the system with context data
//!   ([`ContextDragBehavior`], [`ContextDragWithDirectionBehavior`], [`PanelDragBehavior`]) or with
//!   data of a type ([`TypedDragBehavior`]), notifying an [`IDragHandler`];
//! - drop targets that hand the drag events to an [`IDropHandler`] ([`ContextDropBehavior`]), or
//!   execute a command ([`FilesDropBehavior`], [`TextDropBehavior`], [`PanelDropBehavior`],
//!   [`ContentControlFilesDropBehavior`], [`DragDropCommandsBehavior`]); the base of custom drop
//!   handlers ([`DropHandlerBase`], [`BaseTreeViewDropHandler`]);
//! - the preview of dragged files ([`FilesPreviewBehavior`], [`AddPreviewFilesAction`]);
//! - a managed drag and drop inside the application, with a preview window
//!   ([`ManagedContextDragBehavior`], [`ManagedContextDropBehavior`], [`ManagedDragDropService`],
//!   [`DragPreviewService`]).
//!
//! The styles of the reference (`Styles.xaml`: the adorners of the tree view items a drag passes
//! over) are compiled by the build of the crate; an application includes them with
//! `<StyleInclude Source="ferres://FerroUI.Xaml.Interactions.DragAndDrop/Styles.xaml" />`.
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.DragAndDrop` (and
//! `FerroUI.Xaml.Interactions.DragAndDrop.Controls` for the preview window), the first mapped into
//! the default XML namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

// The compiled markup names the types of the crate by the name of the crate, as the markup of any
// other crate does.
extern crate self as ferroui_xaml_interactions_draganddrop;

// `compiled_xaml` (the compiled markup of the styles document) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

mod add_preview_files_action;
#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod behaviors;
mod content_control_files_drop_behavior;
mod context_drag_behavior;
mod context_drag_behavior_base;
mod context_drag_with_direction_behavior;
mod context_drop_behavior;
mod context_drop_behavior_base;
mod contract;
mod drag_and_drop_events_behavior;
mod drag_drop_commands_behavior;
mod drag_drop_context_store;
mod drop_behavior_base;
mod drop_handler_base;
mod files_drop_behavior;
mod files_preview_behavior;
mod managed_drag_drop;
mod markup_types;
mod panel_drag_behavior;
mod panel_drop_behavior;
mod register_types;
mod routed_handlers;
mod rust_paths;
mod text_drop_behavior;
mod typed_drag_behavior;
mod typed_drag_behavior_base;
pub mod values;

pub use add_preview_files_action::AddPreviewFilesAction;
pub use behaviors::BaseTreeViewDropHandler;
pub use content_control_files_drop_behavior::ContentControlFilesDropBehavior;
pub use context_drag_behavior::ContextDragBehavior;
pub use context_drag_behavior_base::{
    ContextDragBehaviorBase, ContextDragBehaviorBaseImpl, ContextDragBehaviorBaseImplExt, ContextDragBehaviorBaseVTable,
};
pub use context_drag_with_direction_behavior::ContextDragWithDirectionBehavior;
pub use context_drop_behavior::ContextDropBehavior;
pub use context_drop_behavior_base::{
    ContextDropBehaviorBase, ContextDropBehaviorBaseImpl, ContextDropBehaviorBaseImplExt, ContextDropBehaviorBaseVTable,
};
pub use contract::{IDragHandler, IDropHandler};
pub use drag_and_drop_events_behavior::{
    DragAndDropEventsBehavior, DragAndDropEventsBehaviorImpl, DragAndDropEventsBehaviorImplExt,
    DragAndDropEventsBehaviorVTable,
};
pub use drag_drop_commands_behavior::DragDropCommandsBehavior;
pub use drop_behavior_base::DropBehaviorBase;
pub use drop_handler_base::DropHandlerBase;
pub use files_drop_behavior::FilesDropBehavior;
pub use files_preview_behavior::FilesPreviewBehavior;
pub use managed_drag_drop::{
    DragPreviewService, DragPreviewWindow, ManagedContextDragBehavior, ManagedContextDropArgs, ManagedContextDropBehavior,
    ManagedDataTransferExtensions, ManagedDataTransferItemExtensions, ManagedDragDropService,
};
pub use panel_drag_behavior::PanelDragBehavior;
pub use panel_drop_behavior::PanelDropBehavior;
pub use register_types::{register_types, ASSEMBLY};
pub use text_drop_behavior::TextDropBehavior;
pub use typed_drag_behavior::TypedDragBehavior;
pub use typed_drag_behavior_base::{
    TypedDragBehaviorBase, TypedDragBehaviorBaseImpl, TypedDragBehaviorBaseImplExt, TypedDragBehaviorBaseVTable,
};

#[cfg(test)]
mod tests;
