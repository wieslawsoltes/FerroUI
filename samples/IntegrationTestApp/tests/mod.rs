//! The tests of the sample (not ports: the upstream sample has no tests of its own; it is the
//! application the user interface automation tests of the upstream project drive).
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `shell` drives the real application headless: it selects every page of the main window,
//! lays out, renders frames through the compositor with Skia, looks for the controls the
//! automation tests find by their automation ids and names, and does with the mouse what a few
//! of those tests do.

mod compiled_markup;
mod shell;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
