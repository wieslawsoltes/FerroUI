//! The Rust emitter of the XAML compiler against the run-time loader: the
//! differential harness of stage E1 (docs/porting/xaml.md, 9.10.2).
//!
//! | File | Contents |
//! |---|---|
//! | `corpus.rs` | the documents and which of them must (not) be eligible; the documents with a class |
//! | `generated.rs` | the emitter's output for the corpus, CHECKED IN |
//! | `generated_handlers/` | the emitter's output for the class documents of the corpus, CHECKED IN |
//! | `event_handlers.rs` | the class documents: both hosts write the checked-in files; a class populated by them and one populated by the run-time loader are the same tree and call the same handlers; a method that is not found |
//! | `differential_tests.rs` | generated output is current; both back ends build equal object trees; registration by URI |
//! | `build_diagnostics.rs` | the diagnostics of a build against the type models: codes, documents, positions, the severities of an EditorConfig file |
//! | `catalog_measure.rs` | an ignored measurement: the documents of the ControlCatalog compiled against the type models, what compiles and what is refused, by reason |
//! | `model_transform.rs` | the transform of the corpus against the build-time type system, measured against the transform against the run-time type system |
//! | `rust_paths_check.rs` | every public Rust path the framework crates record, named from outside them, CHECKED IN |
//! | `rust_paths_tests.rs` | the check file is current; each path names the type it is recorded for |
//!
//! # Why the generated file is checked in
//!
//! The emitter runs in a host that links the framework (its transform uses
//! the run-time type system until the source scanner exists). A build script
//! doing that would compile the framework a second time as a build
//! dependency, on every build of this crate. Instead the output is a
//! reviewable file, and `generated_output_is_up_to_date` fails when the
//! emitter would write something else. To regenerate:
//!
//! ```text
//! cargo test -p ferroui-markup-xaml-tests --lib emitter::differential_tests::regenerate_emitter_output -- --ignored --exact
//! ```
//!
//! rustc compiles `generated.rs` with this crate, so every emitted call is
//! type-checked for real. If a regenerated file ever fails to compile, the
//! emitter wrote Rust it should not have: fix the emitter (or mark the node
//! not eligible), empty the functions and the `DOCUMENTS` table of
//! `generated.rs` by hand, and regenerate.

pub mod corpus;
#[rustfmt::skip]
pub mod generated;
#[rustfmt::skip]
pub mod generated_handlers;
#[rustfmt::skip]
pub mod rust_paths_check;

#[cfg(test)]
mod build_diagnostics;
#[cfg(test)]
mod catalog_measure;
#[cfg(test)]
mod differential_tests;
#[cfg(test)]
mod event_handlers;
#[cfg(test)]
mod model_emit;
#[cfg(test)]
mod model_transform;
#[cfg(test)]
mod repository_documents;
#[cfg(test)]
mod rust_paths_tests;
