//! ferroui-dialogs
//!
//! The managed dialogs: the managed file chooser ([`ManagedFileChooser`]
//! over a [`ManagedFileChooserViewModel`](internal::ManagedFileChooserViewModel)),
//! the storage provider that shows it ([`ManagedStorageProvider`]), the
//! application builder extension that makes it the file picker of every top
//! level ([`ManagedFileDialogExtensions::use_managed_system_dialogs`]) and
//! the about dialog of the framework ([`AboutFerroDialog`]).
//!
//! The control themes of the file chooser are part of the themes
//! (`Controls/ManagedFileChooser.xaml` of the Fluent and the Simple theme).
//! The document of the about dialog and its font are embedded in the crate
//! as assets of the assembly `FerroUI.Dialogs`.
//!
//! ```ignore
//! AppBuilder::configure::<App>().use_platform_detect().use_managed_system_dialogs()
//! ```

mod about_ferro_dialog_xaml;
mod assets;
pub mod internal;
mod managed_file_chooser;
mod managed_file_chooser_overwrite_prompt;
// The storage provider works on the local file system, which the browser does not have (as
// the file-system storage of the base library).
#[cfg(not(target_arch = "wasm32"))]
mod managed_file_dialog_extensions;
mod managed_file_dialog_options;
#[cfg(not(target_arch = "wasm32"))]
mod managed_storage_provider;
mod markup;
mod register_types;
#[cfg(not(target_arch = "wasm32"))]
mod task_completion_source;

pub use about_ferro_dialog_xaml::AboutFerroDialog;
pub use managed_file_chooser::ManagedFileChooser;
pub use managed_file_chooser_overwrite_prompt::ManagedFileChooserOverwritePrompt;
#[cfg(not(target_arch = "wasm32"))]
pub use managed_file_dialog_extensions::ManagedFileDialogExtensions;
pub use managed_file_dialog_options::ManagedFileDialogOptions;
#[cfg(not(target_arch = "wasm32"))]
pub use managed_storage_provider::ManagedStorageProvider;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
