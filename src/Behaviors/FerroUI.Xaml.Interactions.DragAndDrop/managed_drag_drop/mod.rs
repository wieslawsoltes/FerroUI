//! The managed drag and drop (`ManagedDragDrop/`): a drag inside the application, without the
//! drag and drop of the system, with a preview window that follows the pointer.

mod drag_preview_service;
mod drag_preview_window;
pub(crate) mod managed_context_drag_behavior;
mod managed_context_drop_behavior;
pub(crate) mod managed_data_transfer;
mod managed_data_transfer_extensions;
mod managed_drag_drop_service;

pub use drag_preview_service::DragPreviewService;
pub use drag_preview_window::DragPreviewWindow;
pub use managed_context_drag_behavior::ManagedContextDragBehavior;
pub use managed_context_drop_behavior::{ManagedContextDropArgs, ManagedContextDropBehavior};
pub use managed_data_transfer_extensions::{ManagedDataTransferExtensions, ManagedDataTransferItemExtensions};
pub use managed_drag_drop_service::ManagedDragDropService;
