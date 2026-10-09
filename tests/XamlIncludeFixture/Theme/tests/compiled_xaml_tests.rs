//! The compiled markup of the crate is the output of the emitter of Rust
//! source for the documents of [`crate::documents`] (docs/porting/xaml.md, 9.6
//! and 9.7.3). Not a test of upstream.
//!
//! Two hosts run the emitter, and the tests compare each with the emitter run
//! here, in the tests of the crate, where every type of the crate is linked:
//!
//! - the build script (`build.rs`), which writes the modules `compiled_xaml`
//!   and `compiled_xaml_source_info` and the `.xamlmeta` of the crate to
//!   `OUT_DIR`. It registers the types of the crates the crate depends on and
//!   none of this crate; the comparison shows that it compiles the documents
//!   to the text a host with every type compiles them to;
//! - the ignored test below, which writes the checked-in document of the class
//!   of the crate (`compiled_style_with_service_provider.rs` and its
//!   `.xamlmeta`), because a build script cannot link the crate it builds:
//!
//! ```text
//! cargo test -p xaml-include-fixture-theme --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
//! ```

use ferroui_base::platform::register_assets;
use ferroui_base::StaticType;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::{generate_class_file, generate_file, XamlMetadata};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::{DOCUMENTS, SOURCE_INFO_DOCUMENTS, STYLE_WITH_SERVICE_PROVIDER};
use crate::{StyleWithServiceProvider, ASSEMBLY};

const ROOT_URI: &str = "ferres://Tests/";

/// The output of the build script.
const BUILT: &[(&str, &str)] = &[
    ("compiled_xaml.rs", include_str!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml.rs"))),
    ("compiled_xaml_source_info.rs", include_str!(concat!(env!("OUT_DIR"), "/xaml/compiled_xaml_source_info.rs"))),
];

/// What the emitter generates here.
struct Generated {
    /// The files the build script writes, by name.
    built: Vec<(&'static str, String)>,
    /// The `.xamlmeta` of the crate, which the build script writes.
    metadata: String,
    /// The checked-in files, by their path below the crate directory.
    checked_in: Vec<(&'static str, String)>,
}

fn generate() -> Generated {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();

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
    // The compiler picks the constructor as upstream's does: the class has the one constructor
    // `StyleWithServiceProvider(IServiceProvider? sp = null)`.
    let class_file = generate_class_file(
        <StyleWithServiceProvider as StaticType>::TYPE,
        None,
        "::xaml_include_fixture_theme::compiled_style_with_service_provider",
        &[],
    )
    .unwrap_or_else(|reasons| panic!("the document of the class is not eligible:\n{reasons}"));
    assert!(class_file.warnings.is_empty(), "{:?}", class_file.warnings);

    // One `.xamlmeta` for the crate: the documents of both files other crates can include.
    let class_metadata = class_file.metadata(&[]);
    let mut metadata = file.metadata("xaml_include_fixture_theme", "::xaml_include_fixture_theme::compiled_xaml", &[]);
    metadata.documents.extend(class_metadata.documents.iter().cloned());
    Generated {
        built: vec![("compiled_xaml.rs", file.source), ("compiled_xaml_source_info.rs", source_info_file.source)],
        metadata: metadata.to_json(),
        checked_in: vec![
            ("compiled_style_with_service_provider.rs", class_file.source),
            ("compiled_style_with_service_provider.xamlmeta", class_metadata.to_json()),
        ],
    }
}

/// The line of the first difference of two texts, for the message of a failure.
fn first_difference(a: &str, b: &str) -> usize {
    a.lines().zip(b.lines()).position(|(a, b)| a != b).unwrap_or_else(|| a.lines().count().min(b.lines().count())) + 1
}

/// The build script compiles the documents to the text the emitter gives here.
#[test]
fn build_script_output_is_the_emitters() {
    let generated = generate();
    for ((name, built), (_, expected)) in BUILT.iter().zip(&generated.built) {
        assert!(
            built == expected,
            "{name} of the build script differs from the emitter's output in the tests (first difference at line {})",
            first_difference(built, expected)
        );
    }
    // The file of the build also names the files of the crates the crate is built on, which
    // only the build knows (`DEP_<CRATE>_XAML_XAMLMETA`): everything else is compared.
    let metadata = std::fs::read_to_string(env!("FERROUI_XAMLMETA")).expect("the .xamlmeta of the build can be read");
    let mut metadata = XamlMetadata::parse(&metadata).expect("the .xamlmeta of the build is read");
    metadata.dependencies.clear();
    assert!(metadata.to_json() == generated.metadata, "the .xamlmeta of the build script differs from the emitter's in the tests");
}

#[test]
fn compiled_class_document_is_up_to_date() {
    for (path, generated) in generate().checked_in {
        let checked_in = std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|e| panic!("{path} cannot be read: {e}"));
        assert!(
            generated == checked_in,
            "{path} is out of date; regenerate it with \
             `cargo test -p xaml-include-fixture-theme --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`"
        );
    }
}

/// The `.xamlmeta` is the model the emitter writes, and it names the files of the crates the
/// crate is built on, which export their type models (docs/porting/xaml.md, 9.5.13).
#[test]
fn compiled_xaml_metadata_is_readable() {
    let read = XamlMetadata::read(env!("FERROUI_XAMLMETA")).expect("the file can be read");
    let names: Vec<&str> = read.iter().map(|metadata| metadata.name.as_str()).collect();
    assert_eq!(names, ["Tests", "FerroUI.Base", "FerroUI.Controls", "FerroUI.Markup.Xaml"]);
}

#[test]
#[ignore = "writes the generated files; run it to regenerate the checked-in output"]
fn regenerate_compiled_xaml() {
    for (path, generated) in generate().checked_in {
        std::fs::write(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")), generated)
            .unwrap_or_else(|e| panic!("{path} cannot be written: {e}"));
    }
}
