//! The behaviors and triggers of system services (`System/`).

mod clipboard_monitor_behavior;
mod network_status_trigger;

pub use clipboard_monitor_behavior::ClipboardMonitorBehavior;
pub use network_status_trigger::{NetworkStatus, NetworkStatusTrigger};
