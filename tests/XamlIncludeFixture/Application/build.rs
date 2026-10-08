//! Compiles the documents of the crate (`documents::DOCUMENTS`, and
//! `documents::SOURCE_INFO_DOCUMENTS` with `CreateSourceInfo`) to
//! `$OUT_DIR/xaml/` and writes the `.xamlmeta` of the crate
//! (docs/porting/xaml.md, 9.6).
//!
//! The documents include documents of the crate `xaml-include-fixture-theme`
//! and of the two themes. The compiler links such an include through the
//! `.xamlmeta` of the included crate, which Cargo hands to this script as
//! `DEP_<CRATE>_XAML_XAMLMETA` (the `links` key of that crate), and it reads
//! the types the documents name (the classes of the included documents among
//! them) from the registries of the process: the script links the three
//! crates and registers their types.

use ferroui_build::{Build, XamlGroup};
use ferroui_controls::testing::{TestServices, UnitTestApplication};

include!("assembly.rs");

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    // The services the tests of the crate run the emitter with.
    let _app = UnitTestApplication::start(TestServices::styled_window());
    xaml_include_fixture_theme::register_types();
    ferroui_themes_simple::register_types();
    ferroui_themes_fluent::register_types();
    Build::from_env()
        .assembly(&ASSEMBLY)
        .input("assembly.rs")
        .input("documents.rs")
        .compile_group(XamlGroup::new("compiled_xaml").documents(documents::DOCUMENTS))
        .compile_group(
            XamlGroup::new("compiled_xaml_source_info")
                .documents(documents::SOURCE_INFO_DOCUMENTS)
                .create_source_info(true)
                .internal(),
        )
        .run();
}
