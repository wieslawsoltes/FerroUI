//! The file dialogs of GTK, the fallback of a window when the desktop
//! portal has no file chooser.

pub mod gtk;
pub mod gtk_native_file_dialogs;

pub use gtk_native_file_dialogs::GtkSystemDialog;
