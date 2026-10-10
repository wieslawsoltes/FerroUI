//! sample-build
//!
//! The build script of the small samples (not a port: the counterpart of the resource items
//! of the upstream project files and of the upstream markup compiler), as the build script of
//! the catalog does it for the catalog (`samples/ControlCatalog/build.rs`;
//! docs/porting/xaml.md, 9.5.13, 9.5.22 and 9.6), without what only the catalog needs (the
//! placeholder artwork, the separate asset files of the browser, the run-time markup feature).
//!
//! A sample calls [`SampleBuild::run`] from its `build.rs`:
//!
//! ```ignore
//! fn main() {
//!     sample_build::SampleBuild::new("RenderDemo").run();
//! }
//! ```
//!
//! # The documents
//!
//! Every markup document of the crate directory (`**/*.xaml`) is compiled to `$OUT_DIR/xaml/`,
//! each as a group of its own (a document with a class as the document of its class, with the
//! documents of the sample it includes), against the type models: the scan of the sources of
//! the crate and the models of the crates it is built on, which Cargo hands to the script as
//! `DEP_<CRATE>_XAML_XAMLMETA`. A refusal of the compiler fails the build with the diagnostic
//! of the document. Next to the compiled markup the script writes
//! `$OUT_DIR/compiled_classes.rs`: the compiled documents with a class, each with the function
//! that populates an instance of the class (what `initialize_component()` of a class calls).
//!
//! # The assets
//!
//! `$OUT_DIR/assets.rs` holds the table `ASSETS` of the files under `Assets/` as
//! `(rooted path, bytes)` pairs, the table `DOCUMENT_ASSETS` of the compiled documents (a
//! compiled document is not an asset of the assembly, as upstream's compiler removes a compiled
//! resource; a test build has them for the tests that load every document with the run-time
//! loader too), the documents with the classes they name (`DOCUMENTS`) and the documents
//! `excluded.txt` lists (`EXCLUDED`).
//!
//! # The tests
//!
//! `$OUT_DIR/document_tests.rs` holds one test per document (`document_<name>`: it loads
//! through the run-time loader) and one per document with a class (`class_<name>`: the class
//! constructs, populates itself from its document and is shown in a window); a test of a
//! document `excluded.txt` lists is ignored with the reason of the list.
//! `$OUT_DIR/compiled_document_tests.rs` holds one test per compiled document with a class
//! (`compiled_<name>`: the class populated by its compiled markup is the tree of the class
//! populated by the run-time loader).

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::model::AssemblyModel;
use ferroui_build::{Build, TypeSystem, XamlGroup};

/// The list of the documents that do not load yet: `<file> | [page:] <reason>` per line.
const EXCLUDED_LIST: &str = "excluded.txt";

/// Directories of a sample that hold no documents and no assets.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples", "build"];

/// The build of a sample.
pub struct SampleBuild {
    assembly_name: String,
    default_compile_bindings: bool,
    not_compared: Vec<(String, String)>,
    refused: Vec<(String, String)>,
}

impl SampleBuild {
    /// The build of the sample whose assembly is `assembly_name` (`ASSEMBLY.name` of its
    /// `register_types.rs`: the authority of the URIs of its documents and assets).
    pub fn new(assembly_name: &str) -> Self {
        Self { assembly_name: assembly_name.to_string(), default_compile_bindings: true, not_compared: Vec::new(), refused: Vec::new() }
    }

    /// Whether the bindings of a document are compiled when the document does not say
    /// (the compiled-bindings default of the upstream project file; true by default, as the
    /// samples of the upstream repository are built).
    pub fn default_compile_bindings(mut self, value: bool) -> Self {
        self.default_compile_bindings = value;
        self
    }

    /// A compiled document (its path below the directory of the sample) whose comparison with
    /// the run-time loader is not run, with the reason: its test is ignored with it.
    pub fn not_compared(mut self, document: &str, reason: &str) -> Self {
        self.not_compared.push((document.to_string(), reason.to_string()));
        self
    }

    /// A document (its path below the directory of the sample) the compiler refuses, with the
    /// reason of its first error, which names the gap of the framework (`GAPS.md` of the
    /// sample): the build does not compile it, it stays an asset of the assembly, and its
    /// generated tests are ignored with the reason. The build compiles every other document
    /// and fails if one of them is refused.
    pub fn refused(mut self, document: &str, reason: &str) -> Self {
        self.refused.push((document.to_string(), reason.to_string()));
        self
    }

    /// Compiles the documents and writes the tables and the tests.
    pub fn run(self) {
        let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

        let mut assets = Vec::new();
        collect(&root, &root, &mut assets);
        assets.sort();

        let documents: Vec<(String, String)> = assets
            .iter()
            .filter(|(asset_path, _)| asset_path.ends_with(".xaml"))
            .map(|(asset_path, path)| (asset_path.clone(), read(path)))
            .collect();
        for (name, _) in self.not_compared.iter().chain(&self.refused) {
            assert!(
                documents.iter().any(|(document, _)| document.strip_prefix('/') == Some(name.as_str())),
                "the build script names {name}, which is not a document of the crate"
            );
        }
        let compiled = self.compile_documents(&out_dir, &documents);

        let mut text = String::from("pub(crate) static ASSETS: &[(&str, &[u8])] = &[\n");
        let mut document_assets = String::new();
        for (asset_path, path) in &assets {
            let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
            println!("cargo::rerun-if-changed={}", path.display());
            let table = if compiled.documents.contains(asset_path) { &mut document_assets } else { &mut text };
            writeln!(table, "    ({asset_path:?}, include_bytes!({:?})),", path.display().to_string()).expect("write");
        }
        text.push_str("];\n");
        text.push_str("#[cfg(test)]\npub(crate) static DOCUMENT_ASSETS: &[(&str, &[u8])] = &[\n");
        text.push_str(&document_assets);
        text.push_str("];\n");

        let mut excluded = read_excluded(&root, &assets);
        for (name, reason) in &self.refused {
            excluded.push(Excluded { path: format!("/{name}"), page_only: false, reason: reason.clone() });
        }
        text.push_str("#[allow(dead_code)]\npub(crate) static EXCLUDED: &[(&str, bool, &str)] = &[\n");
        for entry in &excluded {
            writeln!(text, "    ({:?}, {}, {:?}),", entry.path, entry.page_only, entry.reason).expect("write");
        }
        text.push_str("];\n");

        text.push_str("#[allow(dead_code)]\npub(crate) static DOCUMENTS: &[(&str, Option<&str>)] = &[\n");
        let mut tests = String::new();
        for (asset_path, content) in &documents {
            let class = class_of(content);
            writeln!(text, "    ({asset_path:?}, {class:?}),").expect("write");

            let name = test_name(asset_path);
            let document_reason = excluded.iter().find(|e| e.path == *asset_path && !e.page_only).map(|e| e.reason.as_str());
            let page_reason = excluded.iter().find(|e| e.path == *asset_path).map(|e| e.reason.as_str());
            match document_reason {
                Some(reason) => writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write"),
                None => tests.push_str("#[test]\n"),
            }
            writeln!(tests, "fn document_{name}() {{\n    super::support::document_loads({asset_path:?});\n}}\n").expect("write");
            if class.is_some() {
                match page_reason {
                    Some(reason) => writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write"),
                    None => tests.push_str("#[test]\n"),
                }
                writeln!(tests, "fn class_{name}() {{\n    super::support::class_constructs({asset_path:?});\n}}\n").expect("write");
            }
        }
        text.push_str("];\n");

        let compiled_classes = format!(
            "/// The documents the build compiled, by their rooted asset paths, each with the function\n/// that populates an instance of its class from its compiled markup.\npub(crate) static COMPILED: &[(&str, fn(&::ferroui_base::BoxedValue) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadException>)] = &[\n{}];\n",
            compiled.classes
        );
        // Written only when they changed, so that the crate is not rebuilt for nothing.
        for (file, content) in [
            ("assets.rs", &text),
            ("document_tests.rs", &tests),
            ("compiled_classes.rs", &compiled_classes),
            ("compiled_document_tests.rs", &compiled.tests),
        ] {
            let out = out_dir.join(file);
            if fs::read_to_string(&out).ok().as_deref() != Some(content.as_str()) {
                fs::write(&out, content).unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
            }
        }
    }

    /// Compiles the documents `(rooted asset path, text)`, each as a group of its own: the
    /// build fails with the one diagnostic of a refused document.
    fn compile_documents(&self, out_dir: &Path, documents: &[(String, String)]) -> CompiledMarkup {
        let every: Vec<(&str, &str)> = documents.iter().map(|(path, text)| (path.trim_start_matches('/'), text.as_str())).collect();
        let named: Vec<(&str, &str)> =
            every.iter().copied().filter(|(name, _)| !self.refused.iter().any(|(refused, _)| refused == name)).collect();
        let mut build = Build::from_env().type_system(TypeSystem::Model).default_compile_bindings(self.default_compile_bindings);
        for (name, text) in &named {
            // A document that includes another one of the sample is given every document, of
            // which the compiler takes the ones it includes.
            let includes = text.contains(&format!("ferres://{}/", self.assembly_name));
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
        for (name, text) in &named {
            if class_of(text).is_none() {
                continue;
            }
            let rooted = format!("/{name}");
            let document = model.documents.iter().find(|document| {
                document.uri.split_once("://").and_then(|(_, rest)| rest.split_once('/')).is_some_and(|(_, path)| path.eq_ignore_ascii_case(name))
            });
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
            match self.not_compared.iter().find(|(document, _)| document == name) {
                Some((_, reason)) => writeln!(tests, "#[test]\n#[ignore = {reason:?}]").expect("write"),
                None => tests.push_str("#[test]\n"),
            }
            writeln!(tests, "fn compiled_{}() {{\n    super::compare({rooted:?});\n}}\n", test_name(name)).expect("write");
        }
        CompiledMarkup { documents: named.iter().map(|(name, _)| format!("/{name}")).collect(), classes, tests }
    }
}

/// What the build of the documents wrote for the crate.
struct CompiledMarkup {
    /// The rooted asset paths of the compiled documents.
    documents: BTreeSet<String>,
    /// The entries of the table of `compiled_classes.rs`.
    classes: String,
    /// The text of `compiled_document_tests.rs`.
    tests: String,
}

struct Excluded {
    path: String,
    /// The reason applies to the test of the class only (the document loads).
    page_only: bool,
    reason: String,
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

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
        let is_asset = path.extension().is_some_and(|extension| extension == "xaml") || (parts[0] == "Assets" && !name.starts_with('.'));
        if is_asset {
            found.push((format!("/{}", parts.join("/")), path));
        }
    }
}

fn read_excluded(root: &Path, assets: &[(String, PathBuf)]) -> Vec<Excluded> {
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
        assert!(assets.iter().any(|(path, _)| *path == asset_path), "{EXCLUDED_LIST} names {asset_path}, which is not a document of the crate");
        let reason = reason.trim();
        let (page_only, reason) = match reason.strip_prefix("page:") {
            Some(reason) => (true, reason.trim()),
            None => (false, reason),
        };
        excluded.push(Excluded { path: asset_path, page_only, reason: reason.to_string() });
    }
    excluded
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
