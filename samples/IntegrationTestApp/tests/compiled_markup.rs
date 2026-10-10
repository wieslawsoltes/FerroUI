//! The classes of the sample, populated by the compiled markup the build script wrote (what
//! the sample runs with), against the same classes populated by the run-time loader from the
//! same documents (docs/porting/xaml.md, 9.5.19 and 9.5.22): `documents` has one test per
//! compiled document with a class, written by the build script.

use super::support::start_application;
use crate::SAMPLE;

mod documents {
    include!(concat!(env!("OUT_DIR"), "/compiled_document_tests.rs"));
}

/// The body of the generated test `compiled_<name>`.
fn compare(path: &str) {
    let _app = start_application();
    sample_testing::documents::compare(&SAMPLE, path);
}

/// Every document of the sample with a class is compiled by the build and has its class in
/// the table of the sample.
#[test]
fn every_document_with_a_class_is_compiled() {
    crate::register_types();
    let mut compiled = 0;
    for (path, class) in SAMPLE.documents {
        if class.is_none() {
            continue;
        }
        assert!(SAMPLE.is_compiled(path), "{path} is not compiled");
        assert!(SAMPLE.find_class(path).is_some(), "{path} has no class");
        compiled += 1;
    }
    // The application, the main window, the two test windows and the twenty pages.
    assert_eq!(compiled, 24, "the documents with a class");
}
