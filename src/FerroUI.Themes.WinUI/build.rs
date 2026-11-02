//! Embeds the markup documents of the theme as assets and compiles them.
//!
//! As the build script of the Fluent theme (`src/FerroUI.Themes.Fluent/build.rs`): every `.xaml`
//! file under the crate directory is a document of the theme. The table is written to
//! `$OUT_DIR/assets.rs` as `(rooted path, bytes)` pairs and registered with the asset loader by
//! `register_types()`, next to the table of the documents `Controls/excluded.txt` leaves out of
//! the theme. Only the documents that are left out of the theme (not compiled) are registered as
//! assets: the compiled documents are answered by the loader table of the compiled markup. The
//! table of every document (`DOCUMENTS`) exists for the tests of the crate alone.
//!
//! It also compiles the documents of the theme (`ferroui_build::Build` against the type models:
//! the scan of the sources of this crate and the models of the crates it is built on) and exports
//! the description of the compiled markup together with the type model of the crate to the build
//! scripts of the crates that depend on the theme, through the `links` key of the manifest
//! (docs/porting/xaml.md, 9.5.13, 9.6.3 and 9.7.3).

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The list of the control theme documents that are embedded but not merged into the theme yet.
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

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets.rs");

    let mut assets = Vec::new();
    collect(&root, &root, &mut assets);
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
    // compiled.
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

    // With the feature `designer-sources`: the control theme documents of the theme (every document
    // under `Controls/` that the theme compiles), by their resource URI and source text, for the
    // definition libraries of designers. Derived from the same table, so a new document is listed.
    text.push_str("#[cfg(any(test, feature = \"designer-sources\"))]\npub(crate) static DESIGNER_DOCUMENTS: &[(&str, &str)] = &[\n");
    for (asset_path, path) in &assets {
        if !asset_path.starts_with("/Controls/") || !compiled(asset_path) {
            continue;
        }
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        writeln!(
            text,
            "    ({:?}, include_str!({:?})),",
            format!("ferres://FerroUI.Themes.WinUI{asset_path}"),
            path.display().to_string()
        )
        .expect("write");
    }
    text.push_str("];\n");

    text.push_str("pub(crate) static EXCLUDED: &[ExcludedDocument] = &[\n");
    for (asset_path, missing_types) in &excluded {
        writeln!(text, "    ExcludedDocument {{ path: {asset_path:?}, missing_types: {missing_types:?} }},").expect("write");
    }
    text.push_str("];\n");

    // The compiled markup of the theme: the document of the class `WinUITheme` and every document it
    // includes, compiled as one group against the type models into `$OUT_DIR/xaml/compiled_xaml.rs`,
    // with the `.xamlmeta` of the crate. The constructor is stated: the markup metadata of the theme
    // declares `new()` next to the constructor that takes the service provider, so the compiler
    // would pick `new()`; the class is constructed as the Fluent theme is.
    let documents: Vec<(String, String)> = assets
        .iter()
        .filter(|(asset_path, _)| compiled(asset_path))
        .map(|(asset_path, path)| {
            let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            (asset_path.trim_start_matches('/').to_string(), text)
        })
        .collect();
    println!("cargo::rerun-if-env-changed=FERROUI_WINUI_THEME_ROOTS");
    let roots = env::var_os("FERROUI_WINUI_THEME_ROOTS").map(|value| {
        let value = value.into_string().expect("FERROUI_WINUI_THEME_ROOTS must be UTF-8");
        let mut roots: Vec<_> = value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        // The constructor looks this up from Rust.
        roots.push("CompactStyles".to_string());
        roots
    });
    compile_theme(documents, roots);

    // Written only when it changed, so that the crate is not rebuilt for nothing.
    if fs::read_to_string(&out).ok().as_deref() != Some(text.as_str()) {
        fs::write(&out, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
    }
}

fn compile_theme(mut documents: Vec<(String, String)>, roots: Option<Vec<String>>) {
    if let Some(roots) = roots {
        ferroui_build::trim_resource_documents(&mut documents, &roots).expect("trim WinUI theme");
    }
    let documents: Vec<_> = documents.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    #[cfg(any(feature = "precompiled-theme", feature = "packed-theme"))]
    write_archive(&documents);

    let build = ferroui_build::Build::from_env();
    #[cfg(any(feature = "precompiled-theme", not(feature = "packed-theme")))]
    let build = {
        let group = ferroui_build::XamlGroup::new("compiled_xaml")
            .documents(&documents)
            .binary_resources(cfg!(feature = "binary-resources"))
            .class_document("WinUITheme.xaml")
            .constructor(ferroui_build::ClassConstructor::ServiceProvider("with_service_provider"));
        build.type_system(ferroui_build::TypeSystem::Model).compile_group(group)
    };
    #[cfg(all(feature = "packed-theme", not(feature = "precompiled-theme")))]
    let build = build.export_metadata()
        .checked_in_metadata("compiled_xaml.xamlmeta")
        .loader("crate::compiled_xaml::try_load");
    build.run();
}

#[cfg(any(feature = "precompiled-theme", feature = "packed-theme"))]
fn write_archive(documents: &[(&str, &str)]) {
    #[cfg(feature = "precompiled-theme")]
    use ferroui_build::compile_paged_archive as encode;
    #[cfg(all(feature = "portable-solid-theme", not(feature = "precompiled-theme")))]
    use ferroui_build::compile_solid_archive as encode;
    #[cfg(not(any(feature = "portable-solid-theme", feature = "precompiled-theme")))]
    use ferroui_build::compile_archive as encode;
    let pack = encode(documents)
        .unwrap_or_else(|error| panic!("cannot compile WinUI theme pack: {error}"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("winui.fxp");
    if fs::read(&out).ok().as_deref() != Some(&pack) {
        fs::write(out, pack).expect("write WinUI theme pack");
    }
}
