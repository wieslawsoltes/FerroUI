//! Compiles the markup documents of the sample, embeds its assets and generates its
//! per-document tests.
//!
//! The counterpart of the resource items of the upstream project file (every markup
//! document, `**/*.xaml`, everything under `Assets/` and `Pages/teapot.bin`) and of its
//! markup compiler (docs/porting/xaml.md, 9.5.13, 9.5.22 and 9.6).
//!
//! # The documents
//!
//! Every document is compiled to `$OUT_DIR/xaml/`, each as a group of its own (a document
//! with a class as the document of its class, with the documents of the sample it includes),
//! against the type models: the scan of the sources of this crate and the models of the
//! crates it is built on, which Cargo hands to this script as `DEP_<CRATE>_XAML_XAMLMETA`.
//! The script links the compiler and no crate for its types. A document the compiler refuses
//! is listed with its reason (`build/compiled_documents.rs`); any other
//! refusal fails the build with the diagnostic of the document. Next to the compiled markup
//! the script writes `$OUT_DIR/compiled_classes.rs`: the compiled documents with a class,
//! each with the function that populates an instance of the class (`markup::load_component`,
//! what `initialize_component()` of a class calls). The loader table of the compiled markup
//! (`compiled_markup::try_load`, registered by `register_types()`) answers a load of a
//! document by its URI.
//!
//! With the feature `runtime-markup` nothing is compiled: every document is an asset, and
//! the classes are populated by the run-time loader, as the sample was before its markup was
//! compiled. What a build of each kind costs is in docs/porting/xaml.md, 9.5.22.
//!
//! # The assets
//!
//! The table is written to `$OUT_DIR/assets.rs` as `(rooted path, bytes)` pairs and
//! registered with the asset loader by `register_types()`, next to the table of the
//! documents `excluded.txt` lists.
//!
//! With the feature `placeholder-branding` (on by default) an asset `Assets/<path>` that has
//! a counterpart `PlaceholderAssets/<path>` is embedded with the content of
//! the counterpart, and the path data of a `StreamGeometry` resource with
//! the key `<key>` in a document is replaced by the content of
//! `PlaceholderAssets/StreamGeometry/<key>.txt`: the artwork of this project,
//! which the sample shows in place of the brand assets of the upstream
//! project.
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
//! # The tests
//!
//! `$OUT_DIR/document_tests.rs` holds one test per document (it loads
//! through the run-time loader) and one per document with a class (the
//! class constructs, populates itself from its document and is shown in a
//! window); a test of a document `excluded.txt` lists is ignored with the
//! reason of the list. The tests of a document start the application
//! `test_applications.txt` names for it, the unit test application of the
//! tests when the document is not listed there.
//! `$OUT_DIR/compiled_document_tests.rs` holds one test per compiled document
//! with a class: the class populated by its compiled markup is the tree of the
//! class populated by the run-time loader (`tests/compiled_markup.rs`).

#[allow(dead_code)]
#[path = "build/compiled_documents.rs"]
mod compiled_documents;
#[path = "build/page_files.rs"]
mod page_files;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::model::AssemblyModel;
use ferroui_build::{Build, TypeSystem, XamlGroup};

/// The list of the documents that do not load yet.
const EXCLUDED_LIST: &str = "excluded.txt";

/// The list of the documents whose generated tests start another
/// application than the unit test application of the tests.
const TEST_APPLICATION_LIST: &str = "test_applications.txt";

/// The applications a line of [`TEST_APPLICATION_LIST`] can name, with the
/// variant of `TestApplication` (`tests/support.rs`) of each.
const TEST_APPLICATIONS: &[(&str, &str)] = &[("unit-test", "UnitTest"), ("catalog", "Catalog")];

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

/// What the build of the documents wrote for the crate.
#[derive(Default)]
struct CompiledMarkup {
    /// The entries of the table of `compiled_classes.rs`.
    classes: String,
    /// The text of `compiled_document_tests.rs`.
    tests: String,
}

/// Compiles the documents `(rooted asset path, text)` but the ones the compiler refuses
/// (`compiled_documents::REFUSED`), each as a group of its own: one refused document then
/// never takes another with it, and the build fails with the one diagnostic. The texts are
/// what the placeholder artwork made of the files, so that the compiled sample shows the
/// artwork its assets would. `catalog_application` lists the documents whose tests start the
/// application of the catalog.
fn compile_documents(out_dir: &Path, documents: &[(String, String)], catalog_application: &[&str]) -> CompiledMarkup {
    for (name, _) in compiled_documents::REFUSED.iter().chain(compiled_documents::NOT_LOADED).chain(compiled_documents::NOT_RUN) {
        assert!(
            documents.iter().any(|(document, _)| document.strip_prefix('/') == Some(*name)),
            "build/compiled_documents.rs names {name}, which is not a document of the crate"
        );
    }
    let named: Vec<(&str, &str)> = documents.iter().map(|(path, text)| (path.trim_start_matches('/'), text.as_str())).collect();
    let compiled: Vec<(&str, &str)> =
        named.iter().copied().filter(|(name, _)| !compiled_documents::REFUSED.iter().any(|(refused, _)| refused == name)).collect();
    let mut build = Build::from_env()
        .type_system(TypeSystem::Model)
        // The upstream sample is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .input("build/compiled_documents.rs");
    for (name, text) in &compiled {
        // A document that includes another one of the sample is given every document, of
        // which the compiler takes the ones it includes.
        let includes = text.contains(&format!("ferres://{ASSEMBLY_NAME}/"));
        let group = XamlGroup::new(&format!("compiled_{}", test_name(name)));
        let group = match (class_of(text).is_some(), includes) {
            (true, true) => group.documents(&named).class_document(name),
            (true, false) => group.documents(&[(*name, *text)]).class_document(name),
            (false, _) => group.documents(&[(*name, *text)]),
        };
        build = build.compile_group(group);
    }
    let outcome = build.execute();
    for line in &outcome.lines {
        println!("{line}");
    }
    if !outcome.errors.is_empty() {
        for error in &outcome.errors {
            eprintln!("error: {error}");
            println!("cargo::error={error}");
        }
        std::process::exit(1);
    }

    // The compiled documents with a class, from the model the build wrote.
    let crate_name = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME").replace('-', "_");
    let model_path = out_dir.join(format!("{crate_name}.xamlmeta"));
    let model = fs::read_to_string(&model_path).map_err(|error| error.to_string()).and_then(|text| AssemblyModel::parse(&text));
    let model = model.unwrap_or_else(|error| panic!("the model of the crate cannot be read: {error}"));
    let own_crate = format!("::{crate_name}::");
    let in_crate = |path: &str| match path.strip_prefix(&own_crate) {
        Some(rest) => format!("crate::{rest}"),
        None => path.to_string(),
    };
    let mut classes = String::new();
    let mut tests = String::new();
    for (name, text) in &compiled {
        if class_of(text).is_none() {
            continue;
        }
        let rooted = format!("/{name}");
        let document = model
            .documents
            .iter()
            .find(|document| document.uri.split_once("://").and_then(|(_, rest)| rest.split_once('/')).is_some_and(|(_, path)| path.eq_ignore_ascii_case(name)));
        let Some((class, populate)) = document.and_then(|document| Some((document.class_rust_path.as_ref()?, document.populate_path.as_ref()?))) else {
            panic!("{name}: the build wrote no function that populates the class of the document");
        };
        writeln!(
            classes,
            "    ({rooted:?}, |root| {populate}(::core::option::Option::None, root.downcast_ref::<::ferroui_base::Ref<{class}>>().expect(\"an instance of the class of the document\"))),",
            populate = in_crate(populate),
            class = in_crate(class)
        )
        .expect("write");
        match compiled_documents::NOT_RUN.iter().find(|(document, _)| document == name) {
            Some((_, reason)) => writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write"),
            None => tests.push_str("#[test]\n"),
        }
        let application = if catalog_application.contains(&rooted.as_str()) { "Catalog" } else { "UnitTest" };
        writeln!(tests, "fn compiled_{}() {{\n    super::compare({rooted:?}, super::TestApplication::{application});\n}}\n", test_name(name)).expect("write");
    }
    CompiledMarkup { classes, tests }
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

    // The application the tests of a document start: `<file> | <application>` per line.
    let test_applications_path = root.join(TEST_APPLICATION_LIST);
    println!("cargo::rerun-if-changed={}", test_applications_path.display());
    let mut test_applications = BTreeMap::new();
    for line in fs::read_to_string(&test_applications_path).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, application) = line.split_once('|').unwrap_or((line, ""));
        let asset_path = format!("/{}", file.trim());
        assert!(
            assets.iter().any(|(path, _)| *path == asset_path),
            "{TEST_APPLICATION_LIST} names {asset_path}, which is not a document of the crate"
        );
        let application = application.trim();
        let variant = TEST_APPLICATIONS
            .iter()
            .find(|(name, _)| *name == application)
            .map(|(_, variant)| *variant)
            .unwrap_or_else(|| panic!("{TEST_APPLICATION_LIST} names the application {application:?} for {asset_path}"));
        test_applications.insert(asset_path, variant);
    }

    // The compiled markup of the documents, as the placeholder artwork left them.
    let compiled = match env::var_os("CARGO_FEATURE_RUNTIME_MARKUP") {
        Some(_) => CompiledMarkup::default(),
        None => {
            let documents: Vec<(String, String)> = assets
                .iter()
                .filter(|(asset_path, _)| asset_path.ends_with(".xaml"))
                .map(|(asset_path, path)| (asset_path.clone(), fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))))
                .collect();
            let catalog_application: Vec<&str> =
                test_applications.iter().filter(|(_, application)| **application == "Catalog").map(|(asset_path, _)| asset_path.as_str()).collect();
            compile_documents(&out_dir, &documents, &catalog_application)
        }
    };

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
        let application = test_applications.get(asset_path).copied().unwrap_or(TEST_APPLICATIONS[0].1);
        let application = format!("super::support::TestApplication::{application}");
        writeln!(
            tests,
            "fn document_{name}() {{\n    super::support::document_loads({asset_path:?}, {application});\n}}\n"
        )
        .expect("write");
        if class.is_some() {
            if let Some(reason) = page_reason {
                writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write");
            } else {
                tests.push_str("#[test]\n");
            }
            writeln!(
                tests,
                "fn class_{name}() {{\n    super::support::class_constructs({asset_path:?}, {application});\n}}\n"
            )
            .expect("write");
        }
    }
    text.push_str("];\n");

    // Written only when they changed, so that the crate is not rebuilt for nothing.
    let page_assets = plan.rust_tables();
    let compiled_classes = format!(
        "/// The documents the build compiled, by their rooted asset paths, each with the function\n/// that populates an instance of its class from its compiled markup.\npub(crate) static COMPILED: &[(&str, fn(&::ferroui_base::BoxedValue) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadException>)] = &[\n{}];\n",
        compiled.classes
    );
    for (file, content) in [
        ("assets.rs", &text),
        ("document_tests.rs", &tests),
        ("page_assets.rs", &page_assets),
        ("compiled_classes.rs", &compiled_classes),
        ("compiled_document_tests.rs", &compiled.tests),
    ] {
        let out = out_dir.join(file);
        if fs::read_to_string(&out).ok().as_deref() != Some(content.as_str()) {
            fs::write(&out, content).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
        }
    }
}
