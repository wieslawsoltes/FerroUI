//! The tests of the generator, ported from `StaticViewLocator.Tests` (one module per test class of
//! the reference, the names of the tests in snake case).
//!
//! The reference runs the generator over a compilation of a source text; these tests write the
//! source text as the root file of a crate in a directory of their own and run the generator of
//! the build script over it (`TestHelpers/StaticViewLocatorGeneratorVerifier.cs`). The reference
//! also compiles the generated source of every run; the generated source of the shapes these tests
//! check is compiled and run by `tests/FerroUI.StaticViewLocator.Tests`, whose build script runs the
//! generator over locators of each kind.
//!
//! A test whose premise Rust does not have (overloads, `ref` parameters, `dynamic`, sealed
//! classes, run-time type tests) keeps its name and checks the Rust counterpart; its comment says
//! how.

mod adapter_contract_tests;
mod adapter_edge_case_tests;
mod adapter_ref_kind_tests;
mod architecture_tests;
mod constructed_generic_mapping_tests;
mod data_template_match_tests;
mod dynamic_signature_tests;
mod options_tests;
mod resolution_mode_tests;
mod sealed_locator_tests;
mod snapshot_tests;

use crate::{Diagnostic, Outcome, StaticViewLocatorBuild};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The name of the crate of the sources of a test.
pub(crate) const CRATE: &str = "test_app";

/// A directory of its own for a test.
fn fixture_directory() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = std::env::temp_dir()
        .join("ferroui-static-view-locator-tests")
        .join(format!("{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the directory of the test is created");
    directory
}

/// Runs the generator over `source`, the root file of the crate `test_app` (`RunGenerator`).
pub(crate) fn run(source: &str) -> Outcome {
    run_with(source, |build| build)
}

/// Runs the generator over `source`, with the options `configure` sets.
pub(crate) fn run_with(source: &str, configure: impl FnOnce(StaticViewLocatorBuild) -> StaticViewLocatorBuild) -> Outcome {
    let directory = fixture_directory();
    std::fs::write(directory.join("lib.rs"), source).expect("the source of the test is written");
    let build = StaticViewLocatorBuild::new(directory.clone(), directory.join("out"), CRATE, Vec::new())
        .write_files(false)
        .reactiveui_available(true);
    let outcome = configure(build).execute();
    let _ = std::fs::remove_dir_all(&directory);
    outcome
}

/// The generated sources of a run without errors (`GetGeneratedSourcesAsync`).
pub(crate) fn generated(source: &str) -> Outcome {
    generated_with(source, |build| build)
}

/// The generated sources of a run without errors, with options.
pub(crate) fn generated_with(source: &str, configure: impl FnOnce(StaticViewLocatorBuild) -> StaticViewLocatorBuild) -> Outcome {
    let outcome = run_with(source, configure);
    let errors: Vec<String> = outcome.errors().map(Diagnostic::to_string).collect();
    assert!(errors.is_empty(), "the generator reported errors:\n{}", errors.join("\n"));
    outcome
}

/// The generated source of the locator file `name`.
pub(crate) fn file<'a>(outcome: &'a Outcome, name: &str) -> &'a str {
    outcome
        .file(name)
        .unwrap_or_else(|| panic!("the generator did not write {name}; it wrote {:?}", outcome.files.iter().map(|file| &file.name).collect::<Vec<_>>()))
}

/// The diagnostics of the code `code`.
pub(crate) fn diagnostics<'a>(outcome: &'a Outcome, code: &str) -> Vec<&'a Diagnostic> {
    outcome.diagnostics.iter().filter(|diagnostic| diagnostic.code == code).collect()
}

/// The one diagnostic of the code `code` (`Assert.Single`).
pub(crate) fn single<'a>(outcome: &'a Outcome, code: &str) -> &'a Diagnostic {
    let found = diagnostics(outcome, code);
    assert_eq!(found.len(), 1, "one {code} is expected; the diagnostics are {:#?}", outcome.diagnostics);
    found[0]
}

/// The entry of `s_views` that maps the view model `view_model` to a view created by
/// `constructor()`, as generated source writes it.
pub(crate) fn view_entry(view_model: &str, constructor: &str) -> String {
    format!(
        "::ferroui_static_view_locator::TypeEntry::new(::ferroui_static_view_locator::generated::TypeId::of::<{view_model}>, || ::ferroui_static_view_locator::upcast({constructor}())),"
    )
}

/// The beginning of the entry of the view model `view_model` in a table.
pub(crate) fn entry_of(view_model: &str) -> String {
    format!("::ferroui_static_view_locator::TypeEntry::new(::ferroui_static_view_locator::generated::TypeId::of::<{view_model}>, ")
}

/// The beginning of the entry of the view model `view_model` in a table of views (a factory).
pub(crate) fn view_entry_of(view_model: &str) -> String {
    format!("{}|| ", entry_of(view_model))
}

/// The entry of `s_missingViews` of the view model `view_model`.
pub(crate) fn missing_entry(view_model: &str, message: &str) -> String {
    format!("{}{message:?}),", entry_of(view_model))
}

/// The declaration of a view: a class of the object model derived from `base` with a constructor
/// without parameters, in a module that imports `UserControl` and the macros of the class model.
pub(crate) fn view(name: &str) -> String {
    view_on(name, "UserControl")
}

/// The declaration of a view derived from `base`.
pub(crate) fn view_on(name: &str, base: &str) -> String {
    format!(
        "pub struct {name} {{ base: {base} }}\nferro_class!({name}: {base});\nimpl {name} {{\n    pub fn new() -> Ref<Self> {{\n        unimplemented!()\n    }}\n}}\n"
    )
}

/// The imports of a module that declares views.
pub(crate) const VIEW_IMPORTS: &str = "use ferroui_base::{ferro_class, Ref};\nuse ferroui_controls::{UserControl, Window};\n";
