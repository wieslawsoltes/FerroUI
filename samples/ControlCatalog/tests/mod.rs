//! The tests of the sample.
//!
//! `documents` holds the generated per-document tests (see `build.rs`):
//! `document_<name>` loads the document through the run-time loader and
//! `class_<name>` constructs its class and shows it in a window. A test of a
//! document `excluded.txt` lists is ignored with the reason of the list.
//! The tests of a document start the application `test_applications.txt`
//! names for it.
//! `gaps` holds the minimal reproductions of the gaps of the framework the
//! list names.

mod allocations;
mod asset_files;
mod auto_complete_box_page;
mod gaps;
mod gaps_a;
mod gaps_b;
mod gaps_c;
mod gaps_d;
mod frame_benchmark;
mod navigation_pages_b;
mod navigation_pages_a;
mod numeric_up_down_page;
mod open_gl_interop_page;
mod page_assets;
mod performance_pages;
mod support;
mod survey;
mod view_models;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
