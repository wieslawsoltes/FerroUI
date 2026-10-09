//! ferroui-build-scan
//!
//! The build-time type model of a crate, the source scanner that fills it and
//! the export of the model through the build script of the crate
//! (docs/porting/xaml.md, 9.5.1, 9.5.13 and 9.6.3). No counterpart upstream:
//! upstream's compiler reads the types of a referenced assembly from its
//! metadata; a Rust crate has none, so a crate writes what its declarations
//! state into its `.xamlmeta` and hands the file to the crates that depend on
//! it.
//!
//! The crate depends on the standard library and on the three crates the
//! scanner reads Rust source with, and on nothing of the framework: the base
//! crate, the controls and the XAML runtime library take it as a build
//! dependency to export their own models, which a crate that depends on the
//! compiler (`ferroui-build`, which links the three) could not be.
//!
//! ```ignore
//! // build.rs of a crate that declares types and has no documents
//! fn main() {
//!     ferroui_build_scan::export::Export::from_env().run();
//! }
//! ```
//!
//! ```toml
//! # Cargo.toml of that crate
//! [package]
//! build = "build.rs"
//! links = "<crate>_xaml"
//!
//! [build-dependencies]
//! ferroui-build-scan = { workspace = true }
//! ```
//!
//! What is here:
//!
//! - [`model`]: the type model of a crate (`AssemblyModel`, `TypeModel`,
//!   `MemberModel`, `RegisteredModel`) and its file, the `.xamlmeta` of format
//!   2, which keeps the documents of format 1 where the compiler reads them;
//! - [`xaml_metadata`]: the part of the file the compiler of a crate reads of
//!   another crate, its compiled documents (format 1);
//! - [`scanner`]: the source scanner, which fills the model from the
//!   declaration macros of the sources of a crate (`scan_crate`), linking
//!   nothing;
//! - [`model_set`]: the models of several crates read together, so that a
//!   type or a property of one crate is found by the path another crate names
//!   it by;
//! - [`call_forms`]: how generated code calls each member a declaration states
//!   by a callable (xaml.md 9.5.3), chosen at the end of a scan and written
//!   into the model;
//! - [`export`]: the scan of the crate whose build script is running, with the
//!   models of its dependencies, written into its `.xamlmeta`.
//!
//! The compiler over the models (`ModelTypeSystem`, `ModelEmitTypes`,
//! `Build::compile_xaml`) is in `ferroui-build`, which exports every module of
//! this crate under its own name.
//!
//! # The host and the target
//!
//! A build script runs on the host whatever the target of the build is, and
//! the scan reads the sources as text: a `cfg` condition is recorded with the
//! declaration it guards and never evaluated, so the model of a crate is the
//! same text for every target and every set of features. A cross build (the
//! browser build of the framework for `wasm32`) builds this crate and the
//! three it depends on for the host once and exports the same files.

#![forbid(unsafe_code)]

pub mod call_forms;
pub mod export;
mod json;
pub mod model;
pub mod model_set;
pub mod scanner;
pub mod xaml_metadata;

/// The XML namespace of the framework, as the base crate states it
/// (`ferroui_base::metadata::FERRO_XML_NAMESPACE`). The scanner reads the
/// `MarkupAssembly` of a crate that names the constant; this crate cannot link the
/// base crate (the base crate scans itself with it), so the text is stated here, and a
/// test of `ferroui-build`, which links both, compares the two.
pub const FERRO_XML_NAMESPACE: &str = "https://github.com/ferroui";

/// The key of the assembly metadata that turns source information on
/// (`ferroui_base::metadata::MarkupAssembly::CREATE_SOURCE_INFO`), stated here for the
/// reason [`FERRO_XML_NAMESPACE`] is.
pub const CREATE_SOURCE_INFO: &str = "FerroXamlCreateSourceInfo";
