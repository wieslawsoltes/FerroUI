//! The tests of the sample. The reference has none; these are the port's own.
//!
//! - `app_view_model_tests`: the search of the view model on the in-memory catalogue, in the
//!   virtual time of a test scheduler (the throttle, the distinct terms, the empty term, the
//!   results and `IsAvailable`).
//! - `crate_details_view_model_tests`: the properties of a crate, the command that opens its page
//!   and the formatting of the number of downloads.
//! - `main_window_tests`: the window built in the unit-test application, with the bindings of its
//!   code-behind and the view located for the items of its list.
//! - `documents` holds the generated per-document tests (`sample-build`): `document_<name>` loads
//!   the document through the run-time loader and `class_<name>` constructs its class and shows
//!   it in a window.
//! - `compiled_markup` compares each class populated by its compiled markup with the same class
//!   populated by the run-time loader.

mod app_view_model_tests;
mod compiled_markup;
mod crate_details_view_model_tests;
mod main_window_tests;
mod support;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/document_tests.rs"));
}
