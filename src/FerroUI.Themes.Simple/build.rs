//! Embeds the markup documents of the theme as assets.
//!
//! The counterpart of the resource items of the upstream project file:
//! every `.xaml` file under the crate directory, plus the string resources
//! linked from the Fluent theme project as `Strings/InvariantResources.xaml`.
//! The table is written to `$OUT_DIR/assets.rs` as `(rooted path, bytes)`
//! pairs and registered with the asset loader by `register_types()`, next
//! to the table of the documents `Controls/excluded.txt` leaves out of the
//! theme. Only the documents that are left out of the theme (not
//! compiled) are registered as assets: the compiled documents are answered
//! by the loader table of the compiled markup, as upstream's compiler
//! removes every compiled resource from the assembly. The table of every
//! document (`DOCUMENTS`) exists for the tests of the crate alone.
//!
//! It also exports the description of the compiled markup of the theme
//! (`compiled_xaml.xamlmeta`, checked in with `compiled_xaml.rs`) together
//! with the type model of the crate, scanned from its sources, to the build
//! scripts of the crates that depend on the theme, through the `links` key of
//! the manifest: the compiler of such a crate links an include of a document
//! of the theme through it and resolves the types of the theme against it
//! (docs/porting/xaml.md, 9.5.13, 9.6.3 and 9.7.3).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Files of other projects that are assets of this one: (path relative to
/// the crate directory, rooted asset path).
const LINKED: &[(&str, &str)] =
    &[("../FerroUI.Themes.Fluent/Strings/InvariantResources.xaml", "/Strings/InvariantResources.xaml")];

/// The list of the control theme documents that are embedded but not merged
/// into the theme yet.
const EXCLUDED_LIST: &str = "Controls/excluded.txt";

/// Directories of the crate that hold no assets.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples"];

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
            if !name.starts_with('.') && !(directory == root && SKIPPED_DIRECTORIES.contains(&name)) {
                collect(root, &path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "xaml") {
            let relative = path.strip_prefix(root).expect("a path under the crate directory");
            let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            found.push((format!("/{}", parts.join("/")), path));
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    // The compiled markup of the theme (the documents of the checked-in `compiled_xaml.xamlmeta`)
    // and the type model of the crate, scanned from its sources, in one file for the crates that
    // include its documents or name its types (`$OUT_DIR/ferroui_themes_simple.xamlmeta`).
    ferroui_build_scan::export::Export::from_env().compiled_markup("compiled_xaml.xamlmeta").run();
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets.rs");

    let mut assets = Vec::new();
    collect(&root, &root, &mut assets);
    for (relative, asset_path) in LINKED {
        let path = root.join(relative);
        println!("cargo::rerun-if-changed={}", path.display());
        assets.push(((*asset_path).to_string(), path));
    }
    assets.sort();

    // The control theme documents that are left out of the theme: `<file> | <missing types>` per line.
    let excluded_list = root.join(EXCLUDED_LIST);
    println!("cargo::rerun-if-changed={}", excluded_list.display());
    let mut excluded = Vec::new();
    for line in fs::read_to_string(&excluded_list).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, missing_types) = line.split_once('|').unwrap_or((line, ""));
        let asset_path = format!("/Controls/{}", file.trim());
        assert!(
            assets.iter().any(|(path, _)| *path == asset_path),
            "{EXCLUDED_LIST} names {asset_path}, which is not a document of the crate"
        );
        excluded.push((asset_path, missing_types.trim().to_string()));
    }

    // The documents the compiled markup of the theme (`compiled_xaml.rs`) is generated from are not
    // registered: every document of the crate except the ones left out of the theme, which are not
    // compiled. Upstream's compiler removes every compiled resource from the assembly (`res.Remove()`
    // in `XamlCompilerTaskExecutor`) and answers a load by URI through the generated `!XamlLoader`
    // (`try_load` of `compiled_xaml.rs`).
    let compiled = |asset_path: &str| !excluded.iter().any(|(path, _)| path == asset_path);

    let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    let mut documents = String::from("#[cfg(test)]\npub(crate) static DOCUMENTS: &[(&str, &[u8])] = &[\n");
    for (asset_path, path) in &assets {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        let entry = format!("    ({asset_path:?}, include_bytes!({:?})),\n", path.display().to_string());
        if !compiled(asset_path) {
            text.push_str(&entry);
        }
        documents.push_str(&entry);
    }
    text.push_str("];\n");
    documents.push_str("];\n");
    text.push_str(&documents);

    text.push_str("pub(crate) static EXCLUDED: &[ExcludedDocument] = &[\n");
    for (asset_path, missing_types) in &excluded {
        writeln!(text, "    ExcludedDocument {{ path: {asset_path:?}, missing_types: {missing_types:?} }},").expect("write");
    }
    text.push_str("];\n");

    // Written only when it changed, so that the crate is not rebuilt for nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        fs::write(&out, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}
