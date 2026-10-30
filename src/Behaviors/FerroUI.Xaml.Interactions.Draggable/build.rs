//! Compiles the styles document of the crate (`Styles.xaml`), and lists it as the asset a build
//! with the feature `document-assets` embeds (docs/porting/xaml.md, 9.5.13, 9.5.20 and 9.6), as
//! the build script of the colour picker does for its themes.
//!
//! The counterpart of the resource item of the reference project (`Styles.xaml` there) and of its
//! markup compiler: the document is compiled to `$OUT_DIR/xaml/compiled_xaml.rs`, and the loader
//! table of the compiled markup answers a load of it by its URI
//! (`ferres://FerroUI.Xaml.Interactions.Draggable/Styles.xaml`).
//!
//! The compiler reads the types from the type models: the one of this crate, scanned from its
//! sources, and the ones of the crates it is built on (the behaviors object model among them),
//! which Cargo hands to this script as `DEP_<CRATE>_XAML_XAMLMETA`.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use ferroui_build::{Build, TypeSystem, XamlGroup};

/// The documents of the crate, by their path below the crate directory.
const DOCUMENTS: &[&str] = &["Styles.xaml"];

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets.rs");

    let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    let mut documents: Vec<(String, String)> = Vec::with_capacity(DOCUMENTS.len());
    for name in DOCUMENTS {
        let path = root.join(name);
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(text, "    (\"/{name}\", include_bytes!({:?})),", path.display().to_string()).expect("write");
        let document = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        documents.push((name.to_string(), document));
    }
    text.push_str("];\n");

    let documents: Vec<(&str, &str)> = documents.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    Build::from_env()
        .type_system(TypeSystem::Model)
        // The reference is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .compile_group(XamlGroup::new("compiled_xaml").documents(&documents))
        .run();

    // Written only when it changed, so that the crate is not rebuilt for nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        fs::write(&out, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}
