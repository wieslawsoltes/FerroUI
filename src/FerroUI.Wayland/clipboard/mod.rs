//! The UI half of the clipboard and of drag and drop (the directory
//! `Clipboard` of the reference): the clipboard of the platform, the data of
//! an offer of another client, the data this application offers, and the
//! drag source.

pub mod wayland_mime_mapper;

#[cfg(target_os = "linux")]
pub mod wayland_clipboard_impl;
#[cfg(target_os = "linux")]
pub(crate) use wayland_clipboard_impl::ClipboardChanges;
#[cfg(target_os = "linux")]
pub mod wayland_data_transfer;
#[cfg(target_os = "linux")]
pub mod wayland_drag_source;
#[cfg(target_os = "linux")]
pub mod wayland_outgoing_transfer;
