//! The libraries of GLib and GTK, opened at run time.

pub mod glib;
pub mod gtk_interop_helper;
pub mod native_library;

pub use gtk_interop_helper::GtkInteropHelper;
