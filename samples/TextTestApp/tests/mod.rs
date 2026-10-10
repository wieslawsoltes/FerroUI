//! The tests of the sample (not ports: the upstream sample has no tests).
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `shell` drives the real application headless: it shows the main window, lays out, renders
//! frames through the compositor with Skia and looks at what the window drew.
//! `gaps` holds the reproductions of the gaps of the framework the sample found (`GAPS.md`).

mod compiled_markup;
mod gaps;
mod shell;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
