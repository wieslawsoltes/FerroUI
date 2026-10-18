//! The tests of the demo: the reference's demo tests and the port's own.
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.

mod compiled_markup;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
