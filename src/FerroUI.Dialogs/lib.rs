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
//! The document of the about dialog is compiled by the build of the crate
//! (`build.rs`: the compiled markup populates the dialog, and the loader
//! table of the crate answers a load of the document by its URI); its font
//! is embedded in the crate as an asset of the assembly `FerroUI.Dialogs`.
//!
//! ```ignore
//! AppBuilder::configure::<App>().use_platform_detect().use_managed_system_dialogs()
//! ```

mod about_ferro_dialog_xaml;
// `compiled_about_ferro_dialog` (the compiled markup of the document of [`AboutFerroDialog`])
// and `compiled_markup` (the embedded assets, the loader table of the crate and `register()`),
// which the build script of the crate generates (docs/porting/xaml.md, 9.5.13 and 9.6).
ferroui_markup_xaml::include_compiled_xaml!();
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
mod register_types;
mod rust_paths;
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
