//! The compiled markup of the theme (`compiled_xaml.rs`, which the theme is
//! loaded from) is the output of the emitter of Rust source for the
//! documents of [`FluentTheme`]: the document of the class and every document
//! it includes, transformed as one group as the run-time loader loads them.
//! The documents of the file are described for crates that include them by
//! `compiled_xaml.xamlmeta` (docs/porting/xaml.md, 9.7.3), generated and
//! checked with it. Not a test of upstream.
//!
//! ```text
//! cargo test -p ferroui-themes-fluent --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
//! ```

use ferroui_base::StaticType;
use ferroui_markup_xaml_loader::rust_emitter::{generate_class_file, ClassConstructor, ClassFile};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use super::support::start_application;
use crate::FluentTheme;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/compiled_xaml.rs");
const METADATA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/compiled_xaml.xamlmeta");

fn generate() -> ClassFile {
    let _app = start_application();
    crate::register_types();
    // The emitter reads the documents of the class as the run-time loader does.
    FerroRuntimeXamlLoader::register_class_document(<FluentTheme as StaticType>::TYPE, FluentTheme::DOCUMENT_URI);
    // The constructor is stated: the markup metadata of the theme declares `new()` next to the
    // constructor that takes the service provider, so the compiler would pick `new()`.
    // Upstream's class has the one constructor `FluentTheme(IServiceProvider? sp = null)`.
    generate_class_file(
        <FluentTheme as StaticType>::TYPE,
        Some(ClassConstructor::ServiceProvider("with_service_provider")),
        "::ferroui_themes_fluent::compiled_xaml",
        &[],
    )
    .unwrap_or_else(|reasons| panic!("the documents of the theme are not eligible:\n{reasons}"))
}

#[test]
fn compiled_xaml_is_up_to_date() {
    let file = generate();
    let metadata = std::fs::read_to_string(METADATA_PATH).expect("compiled_xaml.xamlmeta can be read");
    assert!(
        file.metadata(&[]).to_json() == metadata,
        "compiled_xaml.xamlmeta is out of date; regenerate it with \
         `cargo test -p ferroui-themes-fluent --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`"
    );
    let generated = file.source;
    let checked_in = std::fs::read_to_string(PATH).expect("compiled_xaml.rs can be read");
    if generated != checked_in {
        let line = generated.lines().zip(checked_in.lines()).position(|(a, b)| a != b).map_or_else(
            || generated.lines().count().min(checked_in.lines().count()),
            |index| index,
        );
        panic!(
            "compiled_xaml.rs is out of date (first difference at line {}); regenerate it with \
             `cargo test -p ferroui-themes-fluent --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`",
            line + 1
        );
    }
}

#[test]
#[ignore = "writes compiled_xaml.rs and compiled_xaml.xamlmeta; run it to regenerate the checked-in output"]
fn regenerate_compiled_xaml() {
    let file = generate();
    std::fs::write(PATH, &file.source).expect("compiled_xaml.rs can be written");
    std::fs::write(METADATA_PATH, file.metadata(&[]).to_json()).expect("compiled_xaml.xamlmeta can be written");
}
