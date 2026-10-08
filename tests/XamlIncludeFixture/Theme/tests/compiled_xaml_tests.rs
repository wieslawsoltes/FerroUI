//! The compiled markup of the crate (`compiled_xaml.rs`,
//! `compiled_xaml_source_info.rs`, `compiled_style_with_service_provider.rs`)
//! is the output of the emitter of Rust source for the documents of
//! [`crate::documents`], and `compiled_xaml.xamlmeta` describes it to the crates
//! that include its documents (docs/porting/xaml.md, 9.7.3). Not a test of
//! upstream.
//!
//! ```text
//! cargo test -p xaml-include-fixture-theme --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
//! ```

use ferroui_base::platform::register_assets;
use ferroui_base::{StaticType, TypeInfo};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_class_file, generate_file, ClassConstructor, XamlMetadata};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::{DOCUMENTS, SOURCE_INFO_DOCUMENTS, STYLE_WITH_SERVICE_PROVIDER};
use crate::{StyleWithServiceProvider, ASSEMBLY};

const ROOT_URI: &str = "ferres://Tests/";

/// The generated files, by their path below the crate directory.
fn generate() -> Vec<(&'static str, String)> {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();
    TypeInfo::register_rust_paths(&[(StyleWithServiceProvider::TYPE, "xaml_include_fixture_theme::StyleWithServiceProvider")]);

    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let file = generate_file(ASSEMBLY.name, ROOT_URI, DOCUMENTS, &configuration, &[]);
    let not_eligible: Vec<String> =
        file.documents.iter().filter_map(|(name, reason)| reason.as_ref().map(|reason| format!("{name}: {reason}"))).collect();
    assert!(not_eligible.is_empty(), "documents are not eligible:\n{}", not_eligible.join("\n"));

    let mut source_info = RuntimeXamlLoaderConfiguration::new();
    source_info.local_assembly = Some(&ASSEMBLY);
    source_info.set_create_source_info(true);
    let source_info_file = generate_file(ASSEMBLY.name, ROOT_URI, SOURCE_INFO_DOCUMENTS, &source_info, &[]);
    let not_eligible: Vec<String> = source_info_file
        .documents
        .iter()
        .filter_map(|(name, reason)| reason.as_ref().map(|reason| format!("{name}: {reason}")))
        .collect();
    assert!(not_eligible.is_empty(), "documents are not eligible:\n{}", not_eligible.join("\n"));

    // The emitter reads the document of the class as the run-time loader does.
    let (path, text) = STYLE_WITH_SERVICE_PROVIDER;
    register_assets(ASSEMBLY.name, &[(path, text.as_bytes())]);
    FerroRuntimeXamlLoader::register_class_document(
        <StyleWithServiceProvider as StaticType>::TYPE,
        &format!("ferres://{}{path}", ASSEMBLY.name),
    );
    // Upstream's class has the one constructor `StyleWithServiceProvider(IServiceProvider? sp = null)`.
    let class_file = generate_class_file(
        <StyleWithServiceProvider as StaticType>::TYPE,
        ClassConstructor::ServiceProvider("new"),
        "::xaml_include_fixture_theme::compiled_style_with_service_provider",
        &[],
    )
    .unwrap_or_else(|reasons| panic!("the document of the class is not eligible:\n{reasons}"));

    // One `.xamlmeta` for the crate: the documents of both files other crates can include.
    let mut metadata = file.metadata("xaml_include_fixture_theme", "::xaml_include_fixture_theme::compiled_xaml", &[]);
    metadata.documents.extend(class_file.metadata(&[]).documents);
    vec![
        ("compiled_xaml.rs", file.source),
        ("compiled_xaml_source_info.rs", source_info_file.source),
        ("compiled_style_with_service_provider.rs", class_file.source),
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
             `cargo test -p xaml-include-fixture-theme --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`"
        );
    }
}

/// The `.xamlmeta` is the model the emitter writes.
#[test]
fn compiled_xaml_metadata_is_readable() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/compiled_xaml.xamlmeta");
    let read = XamlMetadata::read(path).expect("the file can be read");
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].name, "Tests");
}

#[test]
#[ignore = "writes the generated files; run it to regenerate the checked-in output"]
fn regenerate_compiled_xaml() {
    for (path, generated) in generate() {
        std::fs::write(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")), generated)
            .unwrap_or_else(|e| panic!("{path} cannot be written: {e}"));
    }
}
