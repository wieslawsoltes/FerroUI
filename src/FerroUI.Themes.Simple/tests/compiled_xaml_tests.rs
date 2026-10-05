//! The compiled markup of the theme (`compiled_xaml.rs`, which the theme is
//! loaded from) is the output of the emitter of Rust source for the
//! documents of [`SimpleTheme`]: the document of the class and every document
//! it includes, transformed as one group as the run-time loader loads them.
//! Not a test of upstream.
//!
//! ```text
//! cargo test -p ferroui-themes-simple --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored
//! ```

use ferroui_base::StaticType;
use ferroui_markup_xaml_loader::rust_emitter::generate_class_file;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use super::support::start_application;
use crate::SimpleTheme;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/compiled_xaml.rs");

fn generate() -> String {
    let _app = start_application();
    crate::register_types();
    // The emitter reads the documents of the class as the run-time loader does.
    FerroRuntimeXamlLoader::register_class_document(<SimpleTheme as StaticType>::TYPE, SimpleTheme::DOCUMENT_URI);
    generate_class_file(<SimpleTheme as StaticType>::TYPE)
        .unwrap_or_else(|reasons| panic!("the documents of the theme are not eligible:\n{reasons}"))
}

#[test]
fn compiled_xaml_is_up_to_date() {
    let generated = generate();
    let checked_in = std::fs::read_to_string(PATH).expect("compiled_xaml.rs can be read");
    if generated != checked_in {
        let line = generated.lines().zip(checked_in.lines()).position(|(a, b)| a != b).map_or_else(
            || generated.lines().count().min(checked_in.lines().count()),
            |index| index,
        );
        panic!(
            "compiled_xaml.rs is out of date (first difference at line {}); regenerate it with \
             `cargo test -p ferroui-themes-simple --lib tests::compiled_xaml_tests::regenerate_compiled_xaml -- --ignored`",
            line + 1
        );
    }
}

#[test]
#[ignore = "writes compiled_xaml.rs; run it to regenerate the checked-in output"]
fn regenerate_compiled_xaml() {
    std::fs::write(PATH, generate()).expect("compiled_xaml.rs can be written");
}
