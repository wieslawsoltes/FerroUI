//! Embeds the assets of the sample and generates its per-document tests.
//!
//! The counterpart of the resource items of the upstream project file:
//! every markup document (`**/*.xaml`), everything under `Assets/` and
//! `Pages/teapot.bin`. The table is written to `$OUT_DIR/assets.rs` as
//! `(rooted path, bytes)` pairs and registered with the asset loader by
//! `register_types()`, next to the table of the documents `excluded.txt`
//! lists.
//!
//! With the feature `placeholder-branding` an asset `Assets/<path>` that has
//! a counterpart `PlaceholderAssets/<path>` is embedded with the content of
//! the counterpart, and the path data of a `StreamGeometry` resource with
//! the key `<key>` in a document is replaced by the content of
//! `PlaceholderAssets/StreamGeometry/<key>.txt`: the neutral artwork the
//! published browser site shows in place of the brand assets of the
//! upstream project.
//!
//! With the feature `separate-assets` the assets other than the markup
//! documents are not embedded: each is copied, byte for byte, to
//! `$OUT_DIR/browser-site/assets/ControlCatalog/<path>` (its rooted asset
//! path below the directory of the assembly), which `scripts/build-browser.sh`
//! puts in the site. The WebAssembly module then carries the code and the
//! documents, and the 24 MB of pictures and fonts are plain files that the
//! host fetches one by one and registers under their own URI. Which files
//! the start-up and each page need follows from a scan of the sources
//! (`build/page_files.rs`); `assets/ControlCatalog.json` lists them: the
//! start-up files, which include what no source names (each such asset is
//! reported as a warning of the build), and the files of each page, which
//! the host page fetches before the catalog creates the page. The split is
//! also written as tables for the tests (`$OUT_DIR/page_assets.rs`), with
//! or without the feature.
//!
//! `$OUT_DIR/document_tests.rs` holds one test per document (it loads
//! through the run-time loader) and one per document with a class (the
//! class constructs, loads its document and is shown in a window); a test
//! of a document `excluded.txt` lists is ignored with the reason of the
//! list.

#[path = "build/page_files.rs"]
mod page_files;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The list of the documents that do not load yet.
const EXCLUDED_LIST: &str = "excluded.txt";

/// The directory of the placeholder artwork of the feature
/// `placeholder-branding`, with the layout of `Assets/`.
const PLACEHOLDER_DIRECTORY: &str = "PlaceholderAssets";

/// Directories of the crate that hold no assets.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples", "build", PLACEHOLDER_DIRECTORY];

/// The name of the assembly of the sample (`ASSEMBLY.name` of
/// `register_types.rs`): the authority of the URIs of its assets.
const ASSEMBLY_NAME: &str = "ControlCatalog";

/// The directory of the browser site that holds the asset files, one
/// directory per assembly, and the manifest of the files next to it.
const SITE_ASSET_DIRECTORY: &str = "assets";

/// Writes `content` to `path` unless it holds it already, so that the site
/// keeps the modification times of what did not change.
fn write_if_changed(path: &Path, content: &[u8]) {
    if fs::read(path).ok().as_deref() != Some(content) {
        fs::create_dir_all(path.parent().expect("a parent directory"))
            .unwrap_or_else(|e| panic!("cannot create the directory of {}: {e}", path.display()));
        fs::write(path, content).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
}

/// Removes the files below `directory` that `keep` does not list, and the
/// directories that are left empty: what an earlier build wrote for the
/// site and this one no longer does.
fn remove_stale(directory: &Path, keep: &BTreeSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return };
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            remove_stale(&path, keep);
            if fs::read_dir(&path).is_ok_and(|mut rest| rest.next().is_none()) {
                fs::remove_dir(&path).unwrap_or_else(|e| panic!("cannot remove {}: {e}", path.display()));
            }
        } else if !keep.contains(&path) {
            fs::remove_file(&path).unwrap_or_else(|e| panic!("cannot remove {}: {e}", path.display()));
        }
    }
}

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

/// Embeds the placeholder artwork in place of the assets it replaces. Every
/// placeholder must replace an asset, so that a renamed asset cannot slip
/// through with its original content.
fn substitute_placeholders(root: &Path, assets: &mut [(String, PathBuf)]) {
    let directory = root.join(PLACEHOLDER_DIRECTORY);
    println!("cargo::rerun-if-changed={}", directory.display());
    let mut placeholders: Vec<String> = fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()))
        .map(|entry| entry.expect("directory entry").file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.') && name != "README.md" && !directory.join(name).is_dir())
        .collect();
    placeholders.sort();

    for name in placeholders {
        let asset_path = format!("/Assets/{name}");
        let entry = assets
            .iter_mut()
            .find(|(path, _)| *path == asset_path)
            .unwrap_or_else(|| panic!("{PLACEHOLDER_DIRECTORY}/{name} replaces no asset: there is no Assets/{name}"));
        entry.1 = directory.join(&name);
    }
}

/// Replaces the path data of the `StreamGeometry` resources named by the
/// files of `PlaceholderAssets/StreamGeometry` in the documents; a changed
/// document is written below `$OUT_DIR/placeholder` and embedded from there.
/// Every file must replace at least one resource.
fn substitute_placeholder_geometries(root: &Path, out_dir: &Path, assets: &mut [(String, PathBuf)]) {
    let directory = root.join(PLACEHOLDER_DIRECTORY).join("StreamGeometry");
    println!("cargo::rerun-if-changed={}", directory.display());
    let mut geometries: Vec<(String, String)> = fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter_map(|path| {
            let key = path.file_name()?.to_str()?.strip_suffix(".txt")?.to_string();
            let data = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            Some((key, data.trim().to_string()))
        })
        .collect();
    geometries.sort();

    let mut replaced = vec![0usize; geometries.len()];
    for (asset_path, path) in assets.iter_mut().filter(|(asset_path, _)| asset_path.ends_with(".xaml")) {
        let mut content = fs::read_to_string(&*path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let mut changed = false;
        for ((key, data), count) in geometries.iter().zip(replaced.iter_mut()) {
            let start_tag = format!("<StreamGeometry x:Key=\"{key}\">");
            let mut from = 0;
            while let Some(start) = content[from..].find(&start_tag).map(|at| from + at + start_tag.len()) {
                let end = start
                    + content[start..]
                        .find("</StreamGeometry>")
                        .unwrap_or_else(|| panic!("{asset_path}: the resource {key} is not closed"));
                content.replace_range(start..end, data);
                from = start + data.len();
                *count += 1;
                changed = true;
            }
        }
        if changed {
            let target = out_dir.join("placeholder").join(asset_path.trim_start_matches('/'));
            fs::create_dir_all(target.parent().expect("a parent directory"))
                .unwrap_or_else(|e| panic!("cannot create the directory of {}: {e}", target.display()));
            if fs::read_to_string(&target).ok().as_deref() != Some(content.as_str()) {
                fs::write(&target, &content).unwrap_or_else(|e| panic!("cannot write {}: {e}", target.display()));
            }
            *path = target;
        }
    }
    for ((key, _), count) in geometries.iter().zip(&replaced) {
        assert!(*count > 0, "{PLACEHOLDER_DIRECTORY}/StreamGeometry/{key}.txt replaces no StreamGeometry resource of a document");
    }
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

    if env::var_os("CARGO_FEATURE_PLACEHOLDER_BRANDING").is_some() {
        substitute_placeholders(&root, &mut assets);
        substitute_placeholder_geometries(&root, &out_dir, &mut assets);
    }

    // The pages that use each asset (see build/page_files.rs).
    let sizes: BTreeMap<String, u64> = assets
        .iter()
        .filter(|(asset_path, _)| !asset_path.ends_with(".xaml"))
        .map(|(asset_path, path)| {
            let size = fs::metadata(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())).len();
            (asset_path.clone(), size)
        })
        .collect();
    let plan = page_files::plan(&root, &sizes);

    let separate_assets = env::var_os("CARGO_FEATURE_SEPARATE_ASSETS").is_some();
    let site = out_dir.join("browser-site");
    let asset_directory = format!("{SITE_ASSET_DIRECTORY}/{ASSEMBLY_NAME}");
    let mut site_files = BTreeSet::new();
    let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    for (asset_path, path) in &assets {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        if separate_assets && !asset_path.ends_with(".xaml") {
            let out = site.join(&asset_directory).join(asset_path.trim_start_matches('/'));
            write_if_changed(&out, &fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())));
            site_files.insert(out);
            continue;
        }
        writeln!(text, "    ({asset_path:?}, include_bytes!({:?})),", path.display().to_string()).expect("write");
    }
    text.push_str("];\n");
    if separate_assets {
        let manifest = site.join(SITE_ASSET_DIRECTORY).join(format!("{ASSEMBLY_NAME}.json"));
        write_if_changed(&manifest, plan.manifest(ASSEMBLY_NAME, &asset_directory, &sizes).as_bytes());
        site_files.insert(manifest);
        remove_stale(&site, &site_files);
        for asset in &plan.unattributed {
            println!("cargo::warning=no source of the catalog names {asset}: it is a start-up file");
        }
        if !plan.unreached.is_empty() {
            println!(
                "cargo::warning={} assets are named only by sources that neither the start-up nor a page reaches ({}): no page waits for them",
                plan.unreached.len(),
                plan.unreached_sources.join(", ")
            );
        }
        for reference in &plan.unresolved {
            println!("cargo::warning=the asset path {reference} names no asset of the catalog");
        }
    }

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
    let page_assets = plan.rust_tables();
    for (file, content) in [("assets.rs", &text), ("document_tests.rs", &tests), ("page_assets.rs", &page_assets)] {
        let out = out_dir.join(file);
        if fs::read_to_string(&out).ok().as_deref() != Some(content.as_str()) {
            fs::write(&out, content).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
        }
    }
}
