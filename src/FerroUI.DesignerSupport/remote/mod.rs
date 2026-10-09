//! The previewer that talks to an IDE over the remote protocol
//! (`FerroUI.DesignerSupport.Remote`).

pub(crate) mod detachable_transport_connection;
pub(crate) mod file_watcher_transport;
pub mod html_transport;
pub(crate) mod previewer_window_impl;
pub(crate) mod previewer_windowing_platform;
pub(crate) mod remote_designer_entry_point;
pub(crate) mod stubs;

#[cfg(test)]
mod remote_rendering_tests;
#[cfg(test)]
pub(crate) mod test_connection;

pub(crate) use previewer_window_impl::PreviewerWindowImpl;
pub use remote_designer_entry_point::{methods, CommandLineArgs, RemoteDesignerEntryPoint, UsageError};
