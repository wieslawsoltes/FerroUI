//! The clipboard over the selections of the X server (the port of the
//! `Selections/Clipboard` directory).

pub mod clipboard_data_reader;
pub mod clipboard_data_transfer;
pub mod clipboard_data_transfer_item;
pub mod clipboard_read_session_factory;
pub mod event_stream_window;
pub mod x11_clipboard_impl;

pub use x11_clipboard_impl::X11ClipboardImpl;
