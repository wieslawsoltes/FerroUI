//! Compiles the documents of the crate to `$OUT_DIR/xaml/` and writes the
//! `.xamlmeta` of the crate, with its type model, for the crates that include
//! its documents (docs/porting/xaml.md, 9.5.13 and 9.6): the documents without
//! a class (`documents::DOCUMENTS`, and `documents::SOURCE_INFO_DOCUMENTS` with
//! `CreateSourceInfo`) and the document of `StyleWithServiceProvider`, a class
//! of this crate.
//!
//! The compiler reads the types from the type models: the one of this crate,
//! scanned from its sources, and the ones of the crates it is built on, which
//! Cargo hands to this script as `DEP_<CRATE>_XAML_XAMLMETA`. The script links
//! the compiler and no crate for its types, so it compiles the document of a
//! class of the crate it builds, and the assembly is the one `lib.rs` states.

use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    let (path, style_with_service_provider) = documents::STYLE_WITH_SERVICE_PROVIDER;
    let class_document = path.trim_start_matches('/');
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
        .compile_group(
            XamlGroup::new("compiled_style_with_service_provider")
                .documents(&[(class_document, style_with_service_provider)])
                .class_document(class_document),
        )
        .run();
}
