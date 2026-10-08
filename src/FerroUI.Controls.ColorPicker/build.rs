//! Embeds the theme documents of the crate.
//!
//! The counterpart of the resource items of the upstream project file
//! (`Themes/**/*.xaml`, embedded as the markup of the assembly). The table
//! is written to `$OUT_DIR/assets.rs` as `(rooted path, bytes)` pairs and
//! registered with the asset loader by `register_types()`.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

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
    for (asset_path, path) in &assets {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(text, "    ({asset_path:?}, include_bytes!({:?})),", path.display().to_string()).expect("write");
    }
    text.push_str("];\n");

    // Written only when it changed, so that the crate is not rebuilt for nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        fs::write(&out, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}
