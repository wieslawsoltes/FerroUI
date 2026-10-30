//! Dock lifecycle, overlay, and application service contracts.

mod i_dock_global_busy_service;

pub use i_dock_global_busy_service::IDockGlobalBusyService;

mod i_window_lifecycle_service;
pub use i_window_lifecycle_service::IWindowLifecycleService;

mod confirmation_request;
mod delegate_command;
pub use confirmation_request::{ConfirmationCloseAction, ConfirmationRequest};
mod dialog_request;
pub use dialog_request::{DialogCloseAction, DialogRequest};
mod i_dock_dialog_content;
pub use i_dock_dialog_content::{DialogContentCloseAction, IDockDialogContent};
mod i_dock_busy_service;
pub use i_dock_busy_service::{BusyAction, IDockBusyService};
mod i_dock_global_confirmation_service;
pub use i_dock_global_confirmation_service::IDockGlobalConfirmationService;
mod i_dock_global_dialog_service;
pub use i_dock_global_dialog_service::IDockGlobalDialogService;
mod i_reloadable;
pub use i_reloadable::IReloadable;
mod dialog_content;
#[cfg(test)]
mod request_tests;
pub use dialog_content::DialogContent;
mod dialog_result;
pub use dialog_result::DialogResult;
mod i_dock_confirmation_service;
pub use i_dock_confirmation_service::IDockConfirmationService;
mod i_dock_dialog_service;
pub use i_dock_dialog_service::{DockDialogServiceExt, IDockDialogService};
mod i_host_overlay_services;
pub use i_host_overlay_services::IHostOverlayServices;

mod i_visual_tree_lifecycle;
pub use i_visual_tree_lifecycle::IVisualTreeLifecycle;
