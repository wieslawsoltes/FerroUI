//! The tests of the gallery.
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>` loads
//! the document through the run-time loader and `class_<name>` constructs its class and shows it
//! in a window. `compiled_markup` compares each class populated by its compiled markup with the
//! same class populated by the run-time loader. `pages` checks the page table, and
//! `page_behaviors` runs the behaviors of some ported pages.

mod compiled_markup;
mod page_behaviors;
mod pages;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
