//! Embeds the assets of the sample and generates its per-document tests.
//!
//! The counterpart of the resource items of the upstream project file:
//! every markup document (`**/*.xaml`), everything under `Assets/` and
//! `Pages/teapot.bin`. The table is written to `$OUT_DIR/assets.rs` as
//! `(rooted path, bytes)` pairs and registered with the asset loader by
//! `register_types()`, next to the table of the documents `excluded.txt`
//! lists.
//!
//! `$OUT_DIR/document_tests.rs` holds one test per document (it loads
//! through the run-time loader) and one per document with a class (the
//! class constructs, loads its document and is shown in a window); a test
//! of a document `excluded.txt` lists is ignored with the reason of the
//! list.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The list of the documents that do not load yet.
const EXCLUDED_LIST: &str = "excluded.txt";

/// Directories of the crate that hold no assets.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples"];

/// Files that are assets whatever their directory.
const ASSET_FILES: &[&str] = &["Pages/teapot.bin"];

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
            continue;
        }
        let relative = path.strip_prefix(root).expect("a path under the crate directory");
        let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let relative = parts.join("/");
        let is_asset = path.extension().is_some_and(|extension| extension == "xaml")
            || (parts[0] == "Assets" && !name.starts_with('.'))
            || ASSET_FILES.contains(&relative.as_str());
        if is_asset {
            found.push((format!("/{relative}"), path));
        }
    }
}

/// The value of the `x:Class` directive of a document.
fn class_of(text: &str) -> Option<String> {
    let start = text.find("x:Class=")? + "x:Class=".len();
    let quote = text[start..].chars().next()?;
    let rest = &text[start + 1..];
    Some(rest[..rest.find(quote)?].to_string())
}

/// `/Pages/ButtonsPage.xaml` as the name of a test: `pages_buttons_page`.
fn test_name(asset_path: &str) -> String {
    let mut name = String::new();
    let stem = asset_path.trim_start_matches('/').trim_end_matches(".xaml");
    let mut previous_lower = false;
    for c in stem.chars() {
        if c == '/' || c == '.' || c == '-' {
            name.push('_');
            previous_lower = false;
        } else if c.is_ascii_uppercase() {
            if previous_lower {
                name.push('_');
            }
            name.push(c.to_ascii_lowercase());
            previous_lower = false;
        } else {
            name.push(c);
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        }
    }
    name
}

struct Excluded {
    path: String,
    /// The reason applies to the test of the class only (the document loads).
    page_only: bool,
    reason: String,
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

    let mut assets = Vec::new();
    collect(&root, &root, &mut assets);
    assets.sort();

    let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    for (asset_path, path) in &assets {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(text, "    ({asset_path:?}, include_bytes!({:?})),", path.display().to_string()).expect("write");
    }
    text.push_str("];\n");

    // The documents that do not load yet: `<file> | [page:] <reason>` per line.
    let excluded_path = root.join(EXCLUDED_LIST);
    println!("cargo::rerun-if-changed={}", excluded_path.display());
    let mut excluded = Vec::new();
    for line in fs::read_to_string(&excluded_path).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, reason) = line.split_once('|').unwrap_or((line, ""));
        let asset_path = format!("/{}", file.trim());
        assert!(
            assets.iter().any(|(path, _)| *path == asset_path),
            "{EXCLUDED_LIST} names {asset_path}, which is not a document of the crate"
        );
        let reason = reason.trim();
        let (page_only, reason) = match reason.strip_prefix("page:") {
            Some(reason) => (true, reason.trim()),
            None => (false, reason),
        };
        excluded.push(Excluded { path: asset_path, page_only, reason: reason.to_string() });
    }

    text.push_str("pub(crate) static EXCLUDED: &[ExcludedDocument] = &[\n");
    for entry in &excluded {
        writeln!(
            text,
            "    ExcludedDocument {{ path: {:?}, page_only: {}, reason: {:?} }},",
            entry.path, entry.page_only, entry.reason
        )
        .expect("write");
    }
    text.push_str("];\n");

    // The documents and the classes they name.
    text.push_str("pub(crate) static DOCUMENTS: &[(&str, Option<&str>)] = &[\n");
    let mut tests = String::new();
    for (asset_path, path) in &assets {
        if !asset_path.ends_with(".xaml") {
            continue;
        }
        let content = fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let class = class_of(&content);
        writeln!(text, "    ({asset_path:?}, {class:?}),").expect("write");

        let name = test_name(asset_path);
        let document_reason = excluded.iter().find(|e| e.path == *asset_path && !e.page_only).map(|e| e.reason.as_str());
        let page_reason = excluded.iter().find(|e| e.path == *asset_path).map(|e| e.reason.as_str());
        if let Some(reason) = document_reason {
            writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write");
        } else {
            tests.push_str("#[test]\n");
        }
        writeln!(tests, "fn document_{name}() {{\n    super::support::document_loads({asset_path:?});\n}}\n").expect("write");
        if class.is_some() {
            if let Some(reason) = page_reason {
                writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write");
            } else {
                tests.push_str("#[test]\n");
            }
            writeln!(tests, "fn class_{name}() {{\n    super::support::class_constructs({asset_path:?});\n}}\n").expect("write");
        }
    }
    text.push_str("];\n");

    // Written only when they changed, so that the crate is not rebuilt for nothing.
    for (file, content) in [("assets.rs", &text), ("document_tests.rs", &tests)] {
        let out = out_dir.join(file);
        if fs::read_to_string(&out).ok().as_deref() != Some(content.as_str()) {
            fs::write(&out, content).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
        }
    }
}
