//! Compiles the pages of the fixture (`documents::PAGES`) to `$OUT_DIR/xaml/`, each as
//! the document of its class, against the type models: the scan of the sources of this
//! crate (which are the sources of the sample, `lib.rs`) and the models the framework
//! crates export (docs/porting/xaml.md, 9.5.13 and 9.6). It is the build a build script of
//! the sample would run for these pages; the documents are read from the sample and are
//! not changed.

use std::env;
use std::fs;
use std::path::PathBuf;

use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let sample = root.join(documents::SAMPLE);
    let mut build = Build::from_env()
        .type_system(TypeSystem::Model)
        // The sample loads its documents with compiled bindings as their default.
        .default_compile_bindings(true)
        .input("documents.rs");
    for page in documents::PAGES {
        let path = sample.join(page.path);
        println!("cargo::rerun-if-changed={}", path.display());
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        build = build.compile_group(XamlGroup::new(page.module).documents(&[(page.path, text.as_str())]).class_document(page.path));
    }
    build.run();
}
