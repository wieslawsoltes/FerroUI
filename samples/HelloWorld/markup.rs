//! The documents of the sample and the table of the classes the documents name.
//!
//! A class with a document is populated by the compiled markup the build of the crate
//! generates for the document (`build.rs`; docs/porting/xaml.md, 9.5.22), and the constructor
//! of the class calls it (`InitializeComponent()`): `initialize_component()`, which
//! [`xaml_class!`] declares over the table of the compiled documents the build writes
//! ([`SAMPLE`]).

use crate::register_types::ASSEMBLY;
use sample_support::Sample;

pub use sample_support::XamlClass;
pub(crate) use sample_support::xaml_class;

// `ASSETS`, `DOCUMENT_ASSETS` (a test build), `DOCUMENTS` and `EXCLUDED` (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

// `COMPILED`: the documents the build compiled, each with the function that populates an
// instance of its class from its compiled markup (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/compiled_classes.rs"));

/// What the sample states about its documents and assets.
pub static SAMPLE: Sample = Sample {
    assembly: &ASSEMBLY,
    register_types: crate::register_types::register_types,
    classes: &[crate::register_types::CLASSES],
    compiled: COMPILED,
    assets: ASSETS,
    #[cfg(test)]
    document_assets: DOCUMENT_ASSETS,
    #[cfg(not(test))]
    document_assets: &[],
    documents: DOCUMENTS,
};
