//! Compiles the theme documents of the crate and embeds them
//! (docs/porting/xaml.md, 9.5.13, 9.5.20 and 9.6).
//!
//! The counterpart of the resource items of the upstream project file
//! (`Themes/**/*.xaml`) and of its markup compiler: the twelve documents are
//! compiled as one group to `$OUT_DIR/xaml/compiled_xaml.rs` (`Fluent.xaml` and
//! `Simple.xaml` merge the resource dictionaries next to them, and a merge is
//! resolved within one compilation). The loader table of the compiled markup
//! answers a load of a public document by its URI.
//!
//! The compiler reads the types from the type models: the one of this crate,
//! scanned from its sources, and the ones of the crates it is built on, which
//! Cargo hands to this script as `DEP_<CRATE>_XAML_XAMLMETA`. The script links
//! the compiler and no crate for its types. The build also writes the
//! `.xamlmeta` of the crate, with its type model and its compiled documents,
//! and Cargo hands the path of the file to the build scripts of the crates
//! that depend on this one (`DEP_FERROUI_CONTROLS_COLOR_PICKER_XAML_XAMLMETA`):
//! an application whose documents name the types of the library and include
//! its styles is linked to the build functions of the documents by its own
//! compiler.
//!
//! The documents are still embedded as assets too (`$OUT_DIR/assets.rs`, as
//! `(rooted path, bytes)` pairs, registered with the asset loader by
//! `register_types()`). Upstream's compiler removes a compiled resource from the
//! assembly; here a document that includes the styles and is loaded by the
//! run-time loader leaves the include to run time only for a document the asset
//! loader has (docs/porting/xaml.md, decision 22), and the include then loads
//! the compiled markup through the loader table. An application that loads its
//! own markup at run time (the ControlCatalog does) needs the assets for that
//! one question; they go when no such application is left.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::{Build, TypeSystem, XamlGroup};

fn collect(root: &Path, directory: &Path, found: &mut Vec<(String, PathBuf)>) {
    println!("cargo::rerun-if-changed={}", directory.display());
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();

    for path in entries {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        if path.is_dir() {
            if !name.starts_with('.') {
                collect(root, &path, found);
            }
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "xaml") {
            let relative = path.strip_prefix(root).expect("a path under the crate directory");
            let parts: Vec<String> =
                relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            found.push((format!("/{}", parts.join("/")), path));
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets.rs");

    let mut assets = Vec::new();
    collect(&root, &root.join("Themes"), &mut assets);
    assets.sort();

    let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    let mut documents: Vec<(String, String)> = Vec::with_capacity(assets.len());
    for (asset_path, path) in &assets {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(text, "    ({asset_path:?}, include_bytes!({:?})),", path.display().to_string()).expect("write");
        let document = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        documents.push((asset_path.trim_start_matches('/').to_string(), document));
    }
    text.push_str("];\n");

    // The compiled markup of the crate: every theme document, as one group, against the type
    // models (the scan of the sources of this crate and the models of the crates it is built on).
    let documents: Vec<(&str, &str)> = documents.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    Build::from_env()
        .type_system(TypeSystem::Model)
        // The upstream project is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .compile_group(XamlGroup::new("compiled_xaml").documents(&documents))
        .run();

    // Written only when it changed, so that the crate is not rebuilt for nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        fs::write(&out, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}
