//! Compiles the documents of the ControlCatalog to `$OUT_DIR/xaml/`, each as a group of
//! its own (a document with a class as the document of its class), against the type
//! models: the scan of the sources of this crate (which are the sources of the sample,
//! `lib.rs`) and the models of the crates the sample is built on (docs/porting/xaml.md,
//! 9.5.13, 9.5.19 and 9.6). It is the build a build script of the sample would run; the
//! documents are read from the sample and are not changed.
//!
//! Which documents: with the feature `catalog` every document of the sample but the ones
//! the compiler refuses (`documents::REFUSED`, each with its reason); without it the pages
//! of `documents::PAGES`. A document that is not compiled is loaded by the run-time
//! loader, as the sample loads it.
//!
//! Next to the compiled markup the script writes:
//!
//! - `compiled_classes.rs`: the compiled documents with a class, each with the function
//!   that populates an instance of the class (`markup::populate_compiled`);
//! - `assets.rs`: the tables of the module `assets` of the sample (the documents and the
//!   other assets of the sample as embedded assets, as the files are: no placeholder artwork
//!   is put in; the documents `excluded.txt` lists; the documents with their classes);
//! - `compiled_document_tests.rs`: one test per compiled document with a class, which
//!   compares the class populated by its compiled markup with the class populated by the
//!   run-time loader (`tests::compare`), in the application `test_applications.txt` of the
//!   sample names for the tests of the document.
//!
//! Two crates the sample is built on have no build script that exports their type model
//! yet (the OpenGL controls, the view model library of the samples): their models are
//! exported here, into the output directory of this script, as the build scripts of the
//! other crates export theirs. The colour picker exports its own, with its compiled
//! documents, which `App.xaml` includes.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::export::{dependencies_from_env, Export};
use ferroui_build::model::AssemblyModel;
use ferroui_build::{Build, TypeSystem, XamlGroup};

#[allow(dead_code)]
#[path = "documents.rs"]
mod documents;

/// The crates whose models this script exports: the directory below the root of the
/// repository and the package.
const EXPORTED: &[(&str, &str)] = &[("src/FerroUI.OpenGL", "ferroui-opengl"), ("samples/MiniMvvm", "mini-mvvm")];

/// The directories of the sample that hold no documents.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples", "build", "PlaceholderAssets"];

/// Files of the sample that are assets whatever their directory.
const ASSET_FILES: &[&str] = &["Pages/teapot.bin"];

/// Every asset of the sample below `directory` (its documents, the files under `Assets/`
/// and [`ASSET_FILES`]: what the build script of the sample embeds), by its path below
/// `root` with `/` separators.
fn documents_below(root: &Path, directory: &Path, found: &mut Vec<(String, PathBuf)>) {
    println!("cargo::rerun-if-changed={}", directory.display());
    let mut entries: Vec<PathBuf> =
        fs::read_dir(directory).unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display())).map(|entry| entry.expect("an entry").path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        if path.is_dir() {
            if !name.starts_with('.') && !(directory == root && SKIPPED_DIRECTORIES.contains(&name.as_str())) {
                documents_below(root, &path, found);
            }
        } else {
            let relative = path.strip_prefix(root).expect("a path below the sample");
            let parts: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
            let relative = parts.join("/");
            let is_asset = path.extension().is_some_and(|extension| extension == "xaml")
                || (parts[0] == "Assets" && !name.starts_with('.'))
                || ASSET_FILES.contains(&relative.as_str());
            if is_asset {
                found.push((relative, path));
            }
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

/// `Pages/ButtonsPage.xaml` as a name of Rust: `pages_buttons_page`.
fn snake(path: &str) -> String {
    let mut name = String::new();
    let mut previous_lower = false;
    for character in path.trim_end_matches(".xaml").chars() {
        if matches!(character, '/' | '.' | '-') {
            name.push('_');
            previous_lower = false;
        } else if character.is_ascii_uppercase() {
            if previous_lower {
                name.push('_');
            }
            name.push(character.to_ascii_lowercase());
            previous_lower = false;
        } else {
            name.push(character);
            previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
        }
    }
    name
}

fn write_if_changed(path: &Path, text: &str) {
    if fs::read_to_string(path).ok().as_deref() != Some(text) {
        fs::write(path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    }
}

fn fail(errors: &[String]) -> ! {
    for error in errors {
        eprintln!("error: {error}");
        println!("cargo::error={error}");
    }
    std::process::exit(1);
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let sample = root.join(documents::SAMPLE);
    let repository = root.join("..").join("..");
    let full = env::var_os("CARGO_FEATURE_CATALOG").is_some();

    let mut files = Vec::new();
    documents_below(&sample, &sample, &mut files);
    let texts: Vec<(String, PathBuf, String)> = files
        .iter()
        .filter(|(name, _)| name.ends_with(".xaml"))
        .cloned()
        .map(|(name, path)| {
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            (name, path, text)
        })
        .collect();
    for (name, _) in documents::REFUSED.iter().chain(documents::NOT_LOADED).chain(documents::NOT_RUN) {
        assert!(texts.iter().any(|(document, _, _)| document.as_str() == *name), "documents.rs names {name}, which is not a document of the sample");
    }
    for name in documents::PAGES {
        assert!(texts.iter().any(|(document, _, _)| document.as_str() == *name), "documents.rs names {name}, which is not a document of the sample");
        assert!(!documents::REFUSED.iter().any(|(refused, _)| refused == name), "{name} is a page of the fixture and a refused document");
    }

    // The models of the crates that do not export theirs.
    let mut dependencies = dependencies_from_env();
    let framework = dependencies.clone();
    for (directory, package) in EXPORTED {
        let model_directory = out_dir.join("models").join(package);
        fs::create_dir_all(&model_directory).unwrap_or_else(|e| panic!("cannot create {}: {e}", model_directory.display()));
        let outcome = Export::new(repository.join(directory), model_directory.clone(), package, framework.clone()).execute();
        for line in outcome.lines.iter().filter(|line| line.starts_with("cargo::rerun-if-changed=") || line.starts_with("cargo::warning=")) {
            println!("{line}");
        }
        if !outcome.errors.is_empty() {
            fail(&outcome.errors);
        }
        dependencies.push(model_directory.join(format!("{}.xamlmeta", package.replace('-', "_"))));
    }

    // The groups: one per compiled document.
    let compiled: Vec<&(String, PathBuf, String)> = texts
        .iter()
        .filter(|(name, _, _)| match full {
            true => !documents::REFUSED.iter().any(|(refused, _)| *refused == name.as_str()),
            false => documents::PAGES.contains(&name.as_str()),
        })
        .collect();
    let borrowed: Vec<(&str, &str)> = texts.iter().map(|(name, _, text)| (name.as_str(), text.as_str())).collect();
    let package = env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME");
    let mut build = Build::new(root.clone(), out_dir.clone(), &package, dependencies)
        .type_system(TypeSystem::Model)
        // The sample loads its documents with compiled bindings as their default.
        .default_compile_bindings(true)
        .input("documents.rs");
    for (name, _, text) in &compiled {
        // A document that includes another one of the sample is given every document, of
        // which the compiler takes the ones it includes.
        let includes = text.contains("ferres://ControlCatalog/");
        let group = XamlGroup::new(&format!("compiled_{}", snake(name)));
        let group = match (text.contains("x:Class="), includes) {
            (true, true) => group.documents(&borrowed).class_document(name),
            (true, false) => group.documents(&[(name.as_str(), text.as_str())]).class_document(name),
            (false, _) => group.documents(&[(name.as_str(), text.as_str())]),
        };
        build = build.compile_group(group);
    }
    let outcome = build.execute();
    for line in &outcome.lines {
        println!("{line}");
    }
    if !outcome.errors.is_empty() {
        fail(&outcome.errors);
    }

    // The compiled documents with a class, from the model the build wrote.
    let model_path = out_dir.join(format!("{}.xamlmeta", package.replace('-', "_")));
    let model = fs::read_to_string(&model_path).map_err(|error| error.to_string()).and_then(|text| AssemblyModel::parse(&text));
    let model = model.unwrap_or_else(|error| panic!("the model of the crate cannot be read: {error}"));
    let own_crate = format!("::{}::", package.replace('-', "_"));
    let in_crate = |path: &str| match path.strip_prefix(&own_crate) {
        Some(rest) => format!("crate::{rest}"),
        None => path.to_string(),
    };
    let mut classes = String::from(
        "/// The documents the build compiled, by their rooted asset paths, each with the function\n/// that populates an instance of its class from its compiled markup.\npub(crate) static COMPILED: &[(&str, fn(&::ferroui_base::BoxedValue) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadException>)] = &[\n",
    );
    // The documents whose tests start the application of the catalog, as the tests of the
    // sample do: `<file> | catalog` per line.
    let applications_path = sample.join("test_applications.txt");
    println!("cargo::rerun-if-changed={}", applications_path.display());
    let catalog_application: Vec<String> = fs::read_to_string(&applications_path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('|'))
        .filter(|(_, application)| application.trim() == "catalog")
        .map(|(file, _)| file.trim().to_string())
        .collect();
    let mut tests = String::new();
    let mut populated = 0;
    for (name, _, text) in &compiled {
        if !text.contains("x:Class=") {
            continue;
        }
        let rooted = format!("/{name}");
        let document = model
            .documents
            .iter()
            .find(|document| document.uri.split_once("://").and_then(|(_, rest)| rest.split_once('/')).is_some_and(|(_, path)| path.eq_ignore_ascii_case(name)));
        let Some((class, populate)) = document.and_then(|document| Some((document.class_rust_path.as_ref()?, document.populate_path.as_ref()?))) else {
            fail(&[format!("{name}: the build wrote no function that populates the class of the document")]);
        };
        writeln!(
            classes,
            "    ({rooted:?}, |root| {populate}(::core::option::Option::None, root.downcast_ref::<::ferroui_base::Ref<{class}>>().expect(\"an instance of the class of the document\"))),",
            populate = in_crate(populate),
            class = in_crate(class)
        )
        .expect("write");
        populated += 1;
        match documents::NOT_RUN.iter().find(|(document, _)| *document == name.as_str()) {
            Some((_, reason)) => writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write"),
            None => tests.push_str("#[test]\n"),
        }
        let application = if catalog_application.contains(name) { "Catalog" } else { "UnitTest" };
        writeln!(tests, "fn compiled_{}() {{\n    super::compare({rooted:?}, super::TestApplication::{application});\n}}\n", snake(name)).expect("write");
    }
    classes.push_str("];\n");
    write_if_changed(&out_dir.join("compiled_classes.rs"), &classes);
    write_if_changed(&out_dir.join("compiled_document_tests.rs"), &tests);

    // The tables of the module `assets` of the sample.
    let mut assets = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
    for (name, path) in &files {
        let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
        println!("cargo::rerun-if-changed={}", path.display());
        writeln!(assets, "    ({:?}, include_bytes!({:?})),", format!("/{name}"), path.display().to_string()).expect("write");
    }
    assets.push_str("];\n");
    let excluded_path = sample.join("excluded.txt");
    println!("cargo::rerun-if-changed={}", excluded_path.display());
    assets.push_str("pub(crate) static EXCLUDED: &[ExcludedDocument] = &[\n");
    for line in fs::read_to_string(&excluded_path).unwrap_or_default().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (file, reason) = line.split_once('|').unwrap_or((line, ""));
        let reason = reason.trim();
        let (page_only, reason) = match reason.strip_prefix("page:") {
            Some(reason) => (true, reason.trim()),
            None => (false, reason),
        };
        writeln!(assets, "    ExcludedDocument {{ path: {:?}, page_only: {page_only}, reason: {reason:?} }},", format!("/{}", file.trim())).expect("write");
    }
    assets.push_str("];\n");
    assets.push_str("pub(crate) static DOCUMENTS: &[(&str, Option<&str>)] = &[\n");
    for (name, _, text) in &texts {
        writeln!(assets, "    ({:?}, {:?}),", format!("/{name}"), class_of(text)).expect("write");
    }
    assets.push_str("];\n");
    write_if_changed(&out_dir.join("assets.rs"), &assets);

    // What the build did, for whoever reads the output directory.
    let summary = format!(
        "{} documents of the sample, {} compiled ({populated} with a class){}\n",
        texts.len(),
        compiled.len(),
        if full { format!(", {} refused", documents::REFUSED.len()) } else { " (the pages of the fixture; the feature `catalog` compiles every document)".to_string() }
    );
    write_if_changed(&out_dir.join("catalog-summary.txt"), &summary);
}
