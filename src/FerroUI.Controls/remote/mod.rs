//! Remote rendering: a top-level that is rendered into frames sent over a
//! connection of the remote protocol, and a control that shows the frames
//! received from one.

mod remote_server;
mod remote_widget;
mod ui_thread_handle;

pub mod server;

pub use remote_server::RemoteServer;
pub use remote_widget::{RemoteWidget, SizingMode};
pub use ui_thread_handle::UiThreadHandle;

#[cfg(test)]
mod remote_tests;
