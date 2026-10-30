//! Source window lifecycle services.
mod dock_window_lifecycle_service;
pub use dock_window_lifecycle_service::DockWindowLifecycleService;
mod i_activation_bridge;
pub use i_activation_bridge::IActivationBridge;
mod dock_activation_bridge;
pub use dock_activation_bridge::DockActivationBridge;
