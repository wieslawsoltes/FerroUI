//! The tests of the demo: the reference's demo tests and the port's own.
//!
//! `documents` holds the generated per-document tests (`sample-build`): `document_<name>`
//! loads the document through the run-time loader and `class_<name>` constructs its class and
//! shows it in a window.
//! `compiled_markup` compares each class populated by its compiled markup with the same class
//! populated by the run-time loader.
//! `shell` shows every page of the demo window, rendered with Skia.

mod compiled_markup;
mod core_demo_rendering_smoke_tests;
mod shell;
mod support;
mod tree_data_grid_source_extensions_tests;
mod wikipedia_page_view_model_tests;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
