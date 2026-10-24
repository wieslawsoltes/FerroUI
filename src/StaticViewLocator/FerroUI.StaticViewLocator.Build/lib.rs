//! ferroui-static-view-locator-build
//!
//! The generator of `ferroui-static-view-locator`, run by the build script of a crate that
//! declares view locators: a port of the source generator of StaticViewLocator
//! (<https://github.com/wieslawsoltes/StaticViewLocator>, `StaticViewLocatorGenerator`). The plan,
//! the mapping of every feature and the deviations are in `docs/porting/static-view-locator.md`.
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     ferroui_static_view_locator_build::StaticViewLocatorBuild::from_env()
//!         .view_model_namespace_prefixes(["crate::view_models"])
//!         .run();
//! }
//! ```
//!
//! ```toml
//! [build-dependencies]
//! ferroui-static-view-locator-build = "0.4"
//! ```
//!
//! # What a run does
//!
//! 1. Reads the `.xamlmeta` models of the dependencies (`DEP_<LINKS>_XAMLMETA`): the base classes
//!    of the views (`UserControl`, `Window` and the classes derived from them in other crates) and,
//!    with [`include_referenced_crates`](StaticViewLocatorBuild::include_referenced_crates), their
//!    view models.
//! 2. Scans the sources of the crate with the scanner of the framework (`ferroui-build-scan`): the
//!    classes, their base classes and constructors, the paths of the modules.
//! 3. Reads the items of the sources: the types, the `impl` blocks, the visibility of the modules
//!    and the structs with `#[static_view_locator]`.
//! 4. For each locator, maps its view models to their views and writes
//!    `$OUT_DIR/static_view_locator/<name>.rs`, which `#[static_view_locator]` includes.
//! 5. Prints the diagnostics (an error fails the build) and `cargo::rerun-if-changed` for every
//!    file it read.
//!
//! The options of the reference's MSBuild properties are the setters of
//! [`StaticViewLocatorBuild`]; the options of its attribute are the options of
//! `#[static_view_locator(..)]`.

#![forbid(unsafe_code)]

pub mod diagnostics;
mod emit;
mod generator;
pub mod options;
mod source;
#[cfg(test)]
mod tests;
mod universe;

pub use diagnostics::{Diagnostic, Severity};
pub use generator::ResolutionMode;
pub use options::{GeneratorOptions, ReplacementRule};

use ferroui_build_scan::export::{dependencies_from_env, source_root, write_if_changed};
use ferroui_build_scan::model::AssemblyModel;
use ferroui_build_scan::model_set::ModelSet;
use ferroui_build_scan::scanner::{scan_crate, ScanOptions};
use source::SourceCrate;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use universe::Universe;

/// The directory below `OUT_DIR` the generated files are written to; `#[static_view_locator]`
/// includes them from there.
pub const OUTPUT_DIRECTORY: &str = "static_view_locator";

/// A generated file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedFile {
    /// The name of the file below [`OUTPUT_DIRECTORY`] (`ViewLocator.rs`).
    pub name: String,
    pub text: String,
}

/// What a run did.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// The `cargo::` lines, in order (the diagnostics are not among them).
    pub lines: Vec<String>,
    /// The diagnostics, the errors first.
    pub diagnostics: Vec<Diagnostic>,
    /// The generated files, in the order of the locators.
    pub files: Vec<GeneratedFile>,
}

impl Outcome {
    /// The errors.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|diagnostic| diagnostic.severity == Severity::Error)
    }

    /// The generated file `name`.
    pub fn file(&self, name: &str) -> Option<&str> {
        self.files.iter().find(|file| file.name == name).map(|file| file.text.as_str())
    }
}

/// The generator of the locators of a crate, run by its build script.
pub struct StaticViewLocatorBuild {
    manifest_dir: PathBuf,
    out_dir: PathBuf,
    crate_name: String,
    dependencies: Vec<PathBuf>,
    source_root: Option<PathBuf>,
    options: GeneratorOptions,
    reactiveui_available: bool,
    write_files: bool,
}

impl StaticViewLocatorBuild {
    /// The generator of the crate whose build script is running: `CARGO_MANIFEST_DIR`, `OUT_DIR`,
    /// `CARGO_PKG_NAME` (the name of the library when the crate states another one cannot be
    /// read; state it with [`crate_name`](Self::crate_name)), the `.xamlmeta` of every direct
    /// dependency, and whether ferroui-static-view-locator has the feature `reactiveui`
    /// (`DEP_FERROUI_STATIC_VIEW_LOCATOR_REACTIVEUI`).
    ///
    /// # Panics
    /// Panics outside a build script (a variable Cargo sets is missing).
    pub fn from_env() -> Self {
        let variable = |name: &str| env::var_os(name).unwrap_or_else(|| panic!("{name} is not set: not a build script"));
        let mut build = Self::new(
            PathBuf::from(variable("CARGO_MANIFEST_DIR")),
            PathBuf::from(variable("OUT_DIR")),
            &variable("CARGO_PKG_NAME").to_string_lossy(),
            dependencies_from_env(),
        );
        build.reactiveui_available = env::var_os("DEP_FERROUI_STATIC_VIEW_LOCATOR_REACTIVEUI").is_some();
        build
    }

    /// The generator of the package `package_name` in `manifest_dir`, writing below `out_dir`,
    /// with the `.xamlmeta` files of its direct dependencies.
    pub fn new(manifest_dir: PathBuf, out_dir: PathBuf, package_name: &str, dependencies: Vec<PathBuf>) -> Self {
        Self {
            manifest_dir,
            out_dir,
            crate_name: package_name.replace('-', "_"),
            dependencies,
            source_root: None,
            options: GeneratorOptions::default(),
            reactiveui_available: false,
            write_files: true,
        }
    }

    /// The name of the crate as Rust paths spell it, when the library has another name than the
    /// package.
    pub fn crate_name(mut self, name: &str) -> Self {
        self.crate_name = name.replace('-', "_");
        self
    }

    /// The root file of the sources, relative to the crate directory. Without the call:
    /// `src/lib.rs`, `lib.rs`, else `src/main.rs`.
    pub fn source_root(mut self, file: &str) -> Self {
        self.source_root = Some(PathBuf::from(file));
        self
    }

    /// The module paths view models are taken from (`StaticViewLocatorViewModelNamespacePrefixes`).
    pub fn view_model_namespace_prefixes<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, prefixes: I) -> Self {
        self.options.namespace_prefixes = prefixes.into_iter().map(|prefix| prefix.as_ref().to_string()).collect();
        self
    }

    /// `StaticViewLocatorIncludeInternalViewModels`; without an effect (see
    /// [`GeneratorOptions::include_internal_view_models`]).
    pub fn include_internal_view_models(mut self, include: bool) -> Self {
        self.options.include_internal_view_models = include;
        self
    }

    /// Whether the view models of the crates the crate depends on are taken
    /// (`StaticViewLocatorIncludeReferencedAssemblies`).
    pub fn include_referenced_crates(mut self, include: bool) -> Self {
        self.options.include_referenced_crates = include;
        self
    }

    /// More view base types (`StaticViewLocatorAdditionalViewBaseTypes`).
    pub fn additional_view_base_types<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, types: I) -> Self {
        self.options.additional_view_base_types = types.into_iter().map(|type_| type_.as_ref().to_string()).collect();
        self
    }

    /// The namespace rules applied before the defaults (`StaticViewLocatorNamespaceReplacementRules`).
    pub fn namespace_replacement_rules<I: IntoIterator<Item = (F, T)>, F: AsRef<str>, T: AsRef<str>>(mut self, rules: I) -> Self {
        self.options.namespace_replacement_rules = rules.into_iter().map(|(from, to)| ReplacementRule::new(from.as_ref(), to.as_ref())).collect();
        self
    }

    /// The type name rules applied before the default (`StaticViewLocatorTypeNameReplacementRules`).
    pub fn type_name_replacement_rules<I: IntoIterator<Item = (F, T)>, F: AsRef<str>, T: AsRef<str>>(mut self, rules: I) -> Self {
        self.options.type_name_replacement_rules = rules.into_iter().map(|(from, to)| ReplacementRule::new(from.as_ref(), to.as_ref())).collect();
        self
    }

    /// `StaticViewLocatorStripGenericArityFromViewName` (default `true`).
    pub fn strip_generic_arity_from_view_name(mut self, strip: bool) -> Self {
        self.options.strip_generic_arity_from_view_name = strip;
        self
    }

    /// The prefixes removed from the names of trait view models (`StaticViewLocatorInterfacePrefixesToStrip`).
    pub fn interface_prefixes_to_strip<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, prefixes: I) -> Self {
        let prefixes: Vec<String> = prefixes.into_iter().map(|prefix| prefix.as_ref().to_string()).filter(|prefix| !prefix.is_empty()).collect();
        self.options.interface_prefixes_to_strip = if prefixes.is_empty() { vec!["I".to_string()] } else { prefixes };
        self
    }

    /// Sets an option from its text, as an MSBuild property is read
    /// ([`GeneratorOptions::set`]): `property("type_name_replacement_rules", "ViewModel=View,Vm=Page")`.
    pub fn property(mut self, name: &str, value: &str) -> Self {
        if let Err(error) = self.options.set(name, value) {
            panic!("{error}");
        }
        self
    }

    /// All the options at once.
    pub fn options(mut self, options: GeneratorOptions) -> Self {
        self.options = options;
        self
    }

    /// Whether ferroui-static-view-locator has the feature `reactiveui`; [`from_env`](Self::from_env)
    /// reads it from Cargo.
    pub fn reactiveui_available(mut self, available: bool) -> Self {
        self.reactiveui_available = available;
        self
    }

    /// Whether [`execute`](Self::execute) writes the generated files (default `true`); without, the
    /// files are only in the [`Outcome`].
    pub fn write_files(mut self, write: bool) -> Self {
        self.write_files = write;
        self
    }

    /// Runs the generator and prints its lines and diagnostics for Cargo. On an error the process
    /// exits with a failure.
    pub fn run(self) {
        let outcome = self.execute();
        for line in &outcome.lines {
            println!("{line}");
        }
        for diagnostic in &outcome.diagnostics {
            if diagnostic.severity == Severity::Error {
                eprintln!("{diagnostic}");
            }
            println!("{}", diagnostic.cargo_line());
        }
        if outcome.errors().next().is_some() {
            std::process::exit(1);
        }
    }

    /// Runs the generator and returns what [`run`](Self::run) prints, with the generated files.
    pub fn execute(self) -> Outcome {
        let mut outcome = Outcome::default();
        let rerun = |lines: &mut Vec<String>, path: &Path| lines.push(format!("cargo::rerun-if-changed={}", path.display()));
        rerun(&mut outcome.lines, &self.manifest_dir.join("build.rs"));
        rerun(&mut outcome.lines, &self.manifest_dir.join("Cargo.toml"));
        outcome.lines.push("cargo::rerun-if-env-changed=DEP_FERROUI_STATIC_VIEW_LOCATOR_REACTIVEUI".to_string());

        let error = |message: String| Diagnostic {
            code: diagnostics::INVALID_OPTION,
            severity: Severity::Error,
            message,
            file: None,
            line: 0,
        };
        let (models, files) = match ModelSet::read(&self.dependencies) {
            Ok(read) => read,
            Err(message) => {
                outcome.diagnostics.push(error(format!("the .xamlmeta of a dependency cannot be read: {message}")));
                return outcome;
            }
        };
        for file in &files {
            rerun(&mut outcome.lines, file);
        }
        // The crates generated source can name: the direct dependencies.
        let mut direct_crates: BTreeSet<String> = BTreeSet::new();
        for path in &self.dependencies {
            if let Ok(model) = fs::read_to_string(path).map_err(|_| ()).and_then(|text| AssemblyModel::parse(&text).map_err(|_| ())) {
                direct_crates.insert(model.crate_name);
            }
        }
        let root = match source_root(&self.manifest_dir, self.source_root.as_deref()) {
            Ok(root) => root,
            Err(message) => {
                outcome.diagnostics.push(error(message));
                return outcome;
            }
        };
        let models = ModelSet::new(models);
        let scan = scan_crate(&ScanOptions::new(&self.crate_name, &root).with_dependencies(models.models().to_vec()));
        for file in &scan.files {
            rerun(&mut outcome.lines, &file.path);
        }
        let files: Vec<(PathBuf, String)> = scan.files.iter().map(|file| (file.path.clone(), file.module.clone())).collect();
        let source = SourceCrate::read(&self.crate_name, &files);
        outcome.diagnostics.extend(source.diagnostics.iter().cloned());
        let universe = Universe::new(&self.crate_name, &scan, &models, &source, direct_crates);

        // One file per locator: its name, else its id (`GetGeneratedHintName`).
        let mut names: BTreeMap<String, usize> = BTreeMap::new();
        for locator in &source.locators {
            *names.entry(locator.attribute.id.clone().unwrap_or_else(|| locator.name.clone())).or_default() += 1;
        }
        for locator in &source.locators {
            let name = locator.attribute.id.clone().unwrap_or_else(|| locator.name.clone());
            if names[&name] > 1 {
                outcome.diagnostics.push(Diagnostic::error(
                    diagnostics::DUPLICATE_LOCATOR_FILE,
                    format!(
                        "Locator '{}' generates the file '{name}.rs' as another locator of the crate does; give each a distinct id: #[static_view_locator(id = \"..\")]",
                        locator.name
                    ),
                    &locator.location(),
                ));
                continue;
            }
            let mut locator_diagnostics = Vec::new();
            let text = generator::generate_locator(&universe, &self.options, locator, self.reactiveui_available, &mut locator_diagnostics);
            outcome.diagnostics.extend(locator_diagnostics);
            outcome.files.push(GeneratedFile { name: format!("{name}.rs"), text });
        }
        outcome.diagnostics.sort_by_key(|diagnostic| diagnostic.severity);

        if self.write_files {
            let directory = self.out_dir.join(OUTPUT_DIRECTORY);
            for file in &outcome.files {
                if let Err(message) = write_if_changed(&directory.join(&file.name), &file.text) {
                    outcome.diagnostics.push(error(message));
                }
            }
        }
        outcome
    }
}
