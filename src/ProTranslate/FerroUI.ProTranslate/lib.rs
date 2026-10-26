//! ProTranslate localization integrated with FerroUI's native property and binding systems.
//!
//! Services and UI objects are owned by the UI thread. Configure a
//! [`TranslationBindingSource`] explicitly, then use [`TranslateExtension`] or the
//! attached properties on [`Translation`]. Cross-thread producers must post culture
//! changes to the UI dispatcher; no UI object is made `Send` or `Sync`.

extern crate self as ferroui_protranslate;

mod binding_source;
#[cfg(feature = "dependency-injection")]
mod dependency_injection;
mod extensions;
mod register_types;
mod rust_paths;
mod translation;

pub use binding_source::TranslationBindingSource;
#[cfg(feature = "dependency-injection")]
pub use dependency_injection::{
    ProTranslateServiceCollectionExtensions, ProTranslateServiceProviderExtensions,
};
pub use extensions::{FExtension, FormatExtension, TExtension, TranslateExtension};
pub use register_types::{register_types, ASSEMBLY};
pub use translation::Translation;

#[cfg(test)]
mod tests;
