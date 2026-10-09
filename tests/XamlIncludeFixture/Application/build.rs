//! Compiles the documents of the crate (`documents::DOCUMENTS`, and
//! `documents::SOURCE_INFO_DOCUMENTS` with `CreateSourceInfo`) to
//! `$OUT_DIR/xaml/` and writes the `.xamlmeta` of the crate
//! (docs/porting/xaml.md, 9.5.13 and 9.6).
//!
//! The documents include documents of the crate `xaml-include-fixture-theme`
//! and of the two themes, and name types of them. The compiler links such an
//! include through the `.xamlmeta` of the included crate, which Cargo hands to
//! this script as `DEP_<CRATE>_XAML_XAMLMETA` (the `links` key of that crate),
//! and reads the types from the type models in those files and from the scan
//! of the sources of this crate: the script links the compiler and none of
//! the crates for its types.

use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    Build::from_env()
        .type_system(TypeSystem::Model)
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
