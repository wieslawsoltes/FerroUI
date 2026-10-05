//! The Rust emitter of the XAML compiler against the run-time loader: the
//! differential harness of stage E1 (docs/porting/xaml.md, 9.10.2).
//!
//! | File | Contents |
//! |---|---|
//! | `corpus.rs` | the documents and which of them must (not) be eligible |
//! | `generated.rs` | the emitter's output for the corpus, CHECKED IN |
//! | `differential_tests.rs` | generated output is current; both back ends build equal object trees; registration by URI |
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

#[cfg(test)]
mod differential_tests;
