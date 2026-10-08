//! The compiled markup of the crate (the modules `compiled_xaml` and
//! `compiled_xaml_source_info`) is the output of the emitter of Rust source for
//! the documents of [`crate::documents`], compiled with the `.xamlmeta` of the
//! crates whose documents they include (docs/porting/xaml.md, 9.6 and 9.7.3).
//! Not a test of upstream.
//!
//! The build script of the crate writes it to `OUT_DIR`. The test compares that
//! output with the output of the emitter run here, in the tests of the crate:
//! the two hosts register the same crates, and the comparison shows that the
//! build script compiles the documents to the text a test compiles them to,
//! which is how the output was produced while it was checked in.

use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, XamlMetadata};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::{DOCUMENTS, SOURCE_INFO_DOCUMENTS};
use crate::ASSEMBLY;

/// The output of the build script.
const BUILT: &[(&str, &str)] = &[
    ("compiled_xaml.rs", include_str!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml.rs"))),
    ("compiled_xaml_source_info.rs", include_str!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml_source_info.rs"))),
];

/// The `.xamlmeta` of the crate, which the build script wrote, and the files of the
/// crates whose documents the documents include, which it names: the files Cargo
/// handed to the build script as `DEP_<CRATE>_XAML_XAMLMETA` (docs/porting/xaml.md,
/// 9.6.3).
fn metadata() -> Vec<XamlMetadata> {
    XamlMetadata::read(env!("FERROUI_XAMLMETA")).unwrap_or_else(|e| panic!("the .xamlmeta of the crate cannot be read: {e}"))
}

/// The compiled markup of the dependencies.
pub(crate) fn dependencies() -> Vec<XamlMetadata> {
    metadata().into_iter().skip(1).collect()
}

/// The root URI of the documents of the crate.
pub(crate) fn root_uri() -> String {
    format!("ferres://{}/", ASSEMBLY.name)
}

/// The files the build script writes, as the emitter generates them here, by name, and
/// the `.xamlmeta` of the crate without the files of its dependencies.
fn generate() -> (Vec<(&'static str, String)>, XamlMetadata) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();
    let dependencies = dependencies();

    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let file = generate_file(ASSEMBLY.name, &root_uri(), DOCUMENTS, &configuration, &dependencies);
    let mut source_info = RuntimeXamlLoaderConfiguration::new();
    source_info.local_assembly = Some(&ASSEMBLY);
    source_info.set_create_source_info(true);
    let source_info_file = generate_file(ASSEMBLY.name, &root_uri(), SOURCE_INFO_DOCUMENTS, &source_info, &dependencies);
    let not_eligible: Vec<String> = file
        .documents
        .iter()
        .chain(&source_info_file.documents)
        .filter_map(|(name, reason)| reason.as_ref().map(|reason| format!("{name}: {reason}")))
        .collect();
    assert!(not_eligible.is_empty(), "documents are not eligible:\n{}", not_eligible.join("\n"));

    let metadata = file.metadata("xaml_include_fixture_application", "::xaml_include_fixture_application::compiled_xaml", &[]);
    (vec![("compiled_xaml.rs", file.source), ("compiled_xaml_source_info.rs", source_info_file.source)], metadata)
}

/// The line of the first difference of two texts, for the message of a failure.
fn first_difference(a: &str, b: &str) -> usize {
    a.lines().zip(b.lines()).position(|(a, b)| a != b).unwrap_or_else(|| a.lines().count().min(b.lines().count())) + 1
}

/// The build script compiles the documents to the text the emitter gives here.
#[test]
fn build_script_output_is_the_emitters() {
    let (generated, generated_metadata) = generate();
    for ((name, built), (_, expected)) in BUILT.iter().zip(&generated) {
        assert!(
            built == expected,
            "{name} of the build script differs from the emitter's output in the tests (first difference at line {})",
            first_difference(built, expected)
        );
    }
    let built_metadata = metadata().remove(0);
    assert_eq!(built_metadata.name, generated_metadata.name);
    assert_eq!(built_metadata.crate_name, generated_metadata.crate_name);
    assert_eq!(built_metadata.documents, generated_metadata.documents);
}

/// The `.xamlmeta` of the application lists the files of its dependencies, which are
/// read with it: the files of the `DEP_<CRATE>_XAML_XAMLMETA` variables, in the order
/// of the names of the variables.
#[test]
fn compiled_xaml_metadata_reads_its_dependencies() {
    let read = metadata();
    let names: Vec<&str> = read.iter().map(|model| model.name.as_str()).collect();
    assert_eq!(names, [ASSEMBLY.name, "FerroUI.Themes.Fluent", "FerroUI.Themes.Simple", "Tests"]);
    assert_eq!(read[0].dependencies.len(), 3);
}
