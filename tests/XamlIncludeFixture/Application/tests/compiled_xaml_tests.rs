//! The compiled markup of the crate (`compiled_xaml.rs`,
//! `compiled_xaml_source_info.rs`) is the output of the emitter of Rust source
//! for the documents of [`crate::documents`], compiled with the `.xamlmeta` of
//! the crates whose documents they include (docs/porting/xaml.md, 9.7.3);
//! `compiled_xaml.xamlmeta` describes it in turn. Not a test of upstream.
//!
//! ```text
//! cargo test -p xaml-include-fixture-application --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
//! ```

use ferroui_base::TypeInfo;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_file, XamlMetadata};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::{DOCUMENTS, SOURCE_INFO_DOCUMENTS};
use crate::{LocaleCollection, ASSEMBLY};

/// The `.xamlmeta` files of the crates whose documents the documents include, relative
/// to the directory of this crate: what build integration reads from
/// `DEP_<CRATE>_XAML_XAMLMETA` (docs/porting/xaml.md, 9.6.3).
pub(crate) const DEPENDENCIES: &[&str] =
    &["../Theme/compiled_xaml.xamlmeta", "../../../src/FerroUI.Themes.Simple/compiled_xaml.xamlmeta"];

/// The compiled markup of the dependencies.
pub(crate) fn dependencies() -> Vec<XamlMetadata> {
    let mut models = Vec::new();
    for path in DEPENDENCIES {
        for model in XamlMetadata::read(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("the .xamlmeta of a dependency cannot be read: {e}"))
        {
            if !models.iter().any(|known: &XamlMetadata| known.name == model.name) {
                models.push(model);
            }
        }
    }
    models
}

/// The root URI of the documents of the crate.
pub(crate) fn root_uri() -> String {
    format!("ferres://{}/", ASSEMBLY.name)
}

/// The generated files, by their path below the crate directory.
fn generate() -> Vec<(&'static str, String)> {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();
    TypeInfo::register_rust_paths(&[(LocaleCollection::TYPE, "xaml_include_fixture_application::LocaleCollection")]);
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

    let metadata = file.metadata("xaml_include_fixture_application", "::xaml_include_fixture_application::compiled_xaml", DEPENDENCIES);
    vec![
        ("compiled_xaml.rs", file.source),
        ("compiled_xaml_source_info.rs", source_info_file.source),
        ("compiled_xaml.xamlmeta", metadata.to_json()),
    ]
}

#[test]
fn compiled_xaml_is_up_to_date() {
    for (path, generated) in generate() {
        let checked_in = std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("{path} cannot be read: {e}"));
        assert!(
            generated == checked_in,
            "{path} is out of date; regenerate it with \
             `cargo test -p xaml-include-fixture-application --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`"
        );
    }
}

/// The `.xamlmeta` of the application lists the files of its dependencies, which are
/// read with it.
#[test]
fn compiled_xaml_metadata_reads_its_dependencies() {
    let read = XamlMetadata::read(concat!(env!("CARGO_MANIFEST_DIR"), "/compiled_xaml.xamlmeta")).expect("the files can be read");
    let names: Vec<&str> = read.iter().map(|model| model.name.as_str()).collect();
    assert_eq!(names, [ASSEMBLY.name, "Tests", "FerroUI.Themes.Simple"]);
}

#[test]
#[ignore = "writes the generated files; run it to regenerate the checked-in output"]
fn regenerate_compiled_xaml() {
    for (path, generated) in generate() {
        std::fs::write(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")), generated)
            .unwrap_or_else(|e| panic!("{path} cannot be written: {e}"));
    }
}
