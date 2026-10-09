//! Which documents of the sample its build compiles, and what is known of the ones it does
//! not compile or whose compiled markup the tests do not compare with the run-time loader.
//! The build script, the tests of the sample (`tests/compiled_markup.rs`) and the measure of
//! the catalog (`ferroui-markup-xaml-tests`, `emitter::catalog_measure`) read the same lists.
//! A document is named by its path below the directory of the sample, which is its path
//! below the root URI of the assembly.

/// The documents the compiler refuses, each with the reason of its first error
/// (docs/porting/xaml-compiler/HANDOVER.md has the table). The build compiles every other
/// document of the sample and fails if one of them is refused; a document listed here stays
/// an asset of the assembly, as upstream's compiler removes only the resources it compiled.
/// The measure of the catalog is the test that a document listed here is still refused.
pub const REFUSED: &[(&str, &str)] = &[
    ("Pages/OpenGl/OpenGlLeasePage.xaml", "the class of the document is not ported"),
];

/// The compiled documents that neither back end loads in the services of a test, each
/// with the reason: the test of such a document asserts that the compiled markup and the
/// run-time loader fail with the same error, and fails if the document loads.
pub const NOT_LOADED: &[(&str, &str)] = &[];

/// The compiled documents whose comparison is not run, each with the reason (the test of
/// such a document is ignored with it): a document whose load panics in the services of a
/// test, with either back end.
pub const NOT_RUN: &[(&str, &str)] = &[];
