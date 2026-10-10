//! Compiles the theme document of the crate (`HamburgerMenu/HamburgerMenu.xaml`, the
//! counterpart of the resource items of the upstream project file) to `$OUT_DIR/xaml/` and
//! writes the `.xamlmeta` of the crate, with its type model and its compiled document, for the
//! samples that include the document (docs/porting/xaml.md, 9.5.13 and 9.6). The compiler
//! reads the types from the type models: the scan of the sources of this crate and the models
//! of the crates it is built on.
//!
//! A compiled document is not an asset of the assembly, as upstream's compiler removes a
//! compiled resource; with the feature `document-assets` the document is embedded as an asset
//! too (`$OUT_DIR/assets.rs`), which is what lets a document loaded at run time include it.

use ferroui_build::{Build, TypeSystem, XamlGroup};

const DOCUMENT: &str = "HamburgerMenu/HamburgerMenu.xaml";

fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let path = root.join(DOCUMENT);
    println!("cargo::rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Build::from_env()
        .type_system(TypeSystem::Model)
        // The upstream project is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .compile_group(XamlGroup::new("compiled_xaml").documents(&[(DOCUMENT, text.as_str())]))
        .run();

    // The document as an asset, which a build with the feature `document-assets` embeds.
    let canonical = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
    let assets = format!(
        "pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n    ({:?}, include_bytes!({:?})),\n];\n",
        format!("/{DOCUMENT}"),
        canonical.display().to_string()
    );
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets.rs");
    if std::fs::read_to_string(&out).ok().as_deref() != Some(assets.as_str()) {
        std::fs::write(&out, assets).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}
