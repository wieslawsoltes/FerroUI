//! The export of the type model of a crate from its build script
//! (docs/porting/xaml.md, 9.5.13 and 9.6.3): the sources of the crate are
//! scanned, with the models of the crates it depends on, and the model is
//! written to `$OUT_DIR/<crate>.xamlmeta`, whose path Cargo hands to the build
//! scripts of the crates that depend on this one (the `links` key of the
//! manifest and the `cargo::metadata=xamlmeta=<path>` line: they read
//! `DEP_<LINKS>_XAMLMETA`).
//!
//! # What a run does
//!
//! 1. Reads the `.xamlmeta` of every direct dependency and, transitively, the
//!    files they name ([`ModelSet::read`]).
//! 2. Scans the sources of the crate from its root file ([`scan_crate`]).
//! 3. Writes the model, unless the file already holds the same text: the model
//!    of unchanged declarations is the same text, so an edit that changes no
//!    declaration does not make the build scripts of the dependents run.
//! 4. Writes what the run cost to `$OUT_DIR/`[`SCAN_REPORT_FILE`] ([`ScanReport`]).
//! 5. Prints `cargo::rerun-if-changed` for the build script, the manifest,
//!    every file of step 1 and every source file the scan read, and nothing
//!    else: the script runs again when one of them changes. A new source file
//!    is reached through the `mod` item of a file that changed.
//!
//! A crate whose compiled markup is checked in (the themes) exports the
//! documents of its checked-in `.xamlmeta` in the same file
//! ([`Export::compiled_markup`]): the file of the build is then all a crate
//! that depends on it reads, the documents and the types.
//!
//! The text of the model is a function of the sources and of the models of
//! the dependencies alone: the scan reads no environment, records a `cfg`
//! condition instead of evaluating it, and the model is written in the order
//! of the sources.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::model_set::ModelSet;
use crate::xaml_metadata::XamlMetadata;
use crate::scanner::{scan_crate, Scan, ScanOptions, Severity};

/// The file below `OUT_DIR` a run writes its cost to ([`ScanReport`]).
pub const SCAN_REPORT_FILE: &str = "xamlmeta-scan.txt";

/// What a build did: the lines for Cargo and the errors.
pub struct Outcome {
    /// The `cargo::` lines, in order.
    pub lines: Vec<String>,
    /// The errors, each on one line. A build with errors wrote no `.xamlmeta` and no
    /// `mod.rs`.
    pub errors: Vec<String>,
}

/// The `.xamlmeta` of every direct dependency of the crate whose build script is running
/// (`DEP_<LINKS>_XAMLMETA`), in the order of the names of the variables.
pub fn dependencies_from_env() -> Vec<PathBuf> {
    let mut dependencies: Vec<(String, PathBuf)> = env::vars_os()
        .filter_map(|(name, value)| {
            let name = name.into_string().ok()?;
            (name.starts_with("DEP_") && name.ends_with("_XAMLMETA")).then(|| (name, PathBuf::from(value)))
        })
        .collect();
    dependencies.sort();
    dependencies.into_iter().map(|(_, path)| path).collect()
}

/// The root file of the sources of the crate in `manifest_dir`: `stated` (relative to the
/// crate directory), else `src/lib.rs`, else `lib.rs`, else `src/main.rs`.
pub fn source_root(manifest_dir: &Path, stated: Option<&Path>) -> Result<PathBuf, String> {
    let root = match stated {
        Some(root) => manifest_dir.join(root),
        None => ["src/lib.rs", "lib.rs", "src/main.rs"]
            .iter()
            .map(|candidate| manifest_dir.join(candidate))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| {
                format!(
                    "the root file of the sources of the crate is not found in {} (`src/lib.rs`, `lib.rs`, `src/main.rs`): state it with source_root",
                    manifest_dir.display()
                )
            })?,
    };
    if !root.is_file() {
        return Err(format!("the root file of the sources of the crate, {}, is not a file", root.display()));
    }
    Ok(root)
}

/// Writes `text` to `path` unless the file holds it already, so that a build that
/// changed nothing does not make the compiler, or the build script of a dependent crate,
/// run again. Whether the file was written.
pub fn write_if_changed(path: &Path, text: &str) -> Result<bool, String> {
    if fs::read_to_string(path).ok().as_deref() == Some(text) {
        return Ok(false);
    }
    let written = match path.parent() {
        Some(directory) => fs::create_dir_all(directory).and_then(|()| fs::write(path, text)),
        None => fs::write(path, text),
    };
    written.map(|()| true).map_err(|error| format!("{}: {error}", path.display()))
}

/// What the export of a model cost, for whoever judges the price of scanning a crate in
/// its build script. A run writes it to `$OUT_DIR/`[`SCAN_REPORT_FILE`]; nothing of it is
/// printed, and nothing reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// The crate, as Rust paths spell it.
    pub crate_name: String,
    /// The `.xamlmeta` files of the dependencies that were read, and the models in them.
    pub dependency_files: usize,
    pub dependency_models: usize,
    /// The time it took to read them.
    pub read_dependencies: Duration,
    /// The source files the scan read, and the types of the model.
    pub files: usize,
    pub types: usize,
    /// The time the scan took.
    pub scan: Duration,
    /// The size of the text of the model, whether the file was written (it is not when it
    /// holds the same text already) and the time that took.
    pub model_bytes: usize,
    pub model_written: bool,
    pub write: Duration,
    /// The diagnostics of the scan: all of them, and the declarations it did not read.
    pub diagnostics: usize,
    pub not_read: usize,
}

impl ScanReport {
    /// The report as the text of its file, one number per line.
    pub fn text(&self) -> String {
        let seconds = |duration: Duration| format!("{:.3} s", duration.as_secs_f64());
        let mut text = String::new();
        text.push_str(&format!("crate: {}\n", self.crate_name));
        text.push_str(&format!(
            "models of the dependencies: {} read from {} files in {}\n",
            self.dependency_models,
            self.dependency_files,
            seconds(self.read_dependencies)
        ));
        text.push_str(&format!("source files scanned: {}\n", self.files));
        text.push_str(&format!("types: {}\n", self.types));
        text.push_str(&format!("scan: {}\n", seconds(self.scan)));
        text.push_str(&format!(
            "model: {} bytes, {} in {}\n",
            self.model_bytes,
            if self.model_written { "written" } else { "unchanged, not written" },
            seconds(self.write)
        ));
        text.push_str(&format!("diagnostics: {}, of them declarations not read: {}\n", self.diagnostics, self.not_read));
        text.push_str(&format!("total: {}\n", seconds(self.read_dependencies + self.scan + self.write)));
        text
    }

    /// The numbers of `scan`, which took `elapsed`.
    pub fn scanned(&mut self, scan: &Scan, elapsed: Duration) {
        self.files = scan.files.len();
        self.types = scan.model.types.len();
        self.scan = elapsed;
        self.diagnostics = scan.diagnostics.len();
        self.not_read = scan.diagnostics_of(Severity::Error).count();
    }

    /// Writes the report below `out_dir`.
    pub fn write(&self, out_dir: &Path) -> Result<PathBuf, String> {
        let path = out_dir.join(SCAN_REPORT_FILE);
        fs::create_dir_all(out_dir).and_then(|()| fs::write(&path, self.text())).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(path)
    }
}

/// The export of the type model of a crate; see the module documentation.
pub struct Export {
    manifest_dir: PathBuf,
    out_dir: PathBuf,
    crate_name: String,
    dependencies: Vec<PathBuf>,
    source_root: Option<PathBuf>,
    compiled_markup: Vec<PathBuf>,
}

impl Export {
    /// The export of the crate whose build script is running: `CARGO_MANIFEST_DIR`,
    /// `OUT_DIR`, `CARGO_PKG_NAME`, and the `.xamlmeta` of every direct dependency
    /// ([`dependencies_from_env`]).
    ///
    /// # Panics
    /// Panics outside a build script (a variable Cargo sets is missing).
    pub fn from_env() -> Self {
        let variable = |name: &str| env::var_os(name).unwrap_or_else(|| panic!("{name} is not set: not a build script"));
        Self::new(
            PathBuf::from(variable("CARGO_MANIFEST_DIR")),
            PathBuf::from(variable("OUT_DIR")),
            &variable("CARGO_PKG_NAME").to_string_lossy(),
            dependencies_from_env(),
        )
    }

    /// The export of the package `package_name` in `manifest_dir`, writing below `out_dir`,
    /// with the `.xamlmeta` files of its direct dependencies.
    pub fn new(manifest_dir: PathBuf, out_dir: PathBuf, package_name: &str, dependencies: Vec<PathBuf>) -> Self {
        Self { manifest_dir, out_dir, crate_name: package_name.replace('-', "_"), dependencies, source_root: None, compiled_markup: Vec::new() }
    }

    /// The `.xamlmeta` of a checked-in generated file of the crate, relative to the crate
    /// directory (format 1, written by the emitter next to the generated file): its
    /// documents are the documents of the exported file. The file must describe the
    /// assembly of the crate.
    pub fn compiled_markup(mut self, path: &str) -> Self {
        self.compiled_markup.push(self.manifest_dir.join(path));
        self
    }

    /// The root file of the sources of the crate, relative to the crate directory. Without
    /// the call: [`source_root`].
    pub fn source_root(mut self, file: &str) -> Self {
        self.source_root = Some(PathBuf::from(file));
        self
    }

    /// Runs the export and prints its lines for Cargo. On an error every error is printed
    /// and the process exits with a failure.
    pub fn run(self) {
        let outcome = self.execute();
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
    }

    /// Runs the export and returns what [`run`](Self::run) prints.
    pub fn execute(self) -> Outcome {
        let mut lines = Vec::new();
        let rerun = |lines: &mut Vec<String>, path: &Path| lines.push(format!("cargo::rerun-if-changed={}", path.display()));
        rerun(&mut lines, &self.manifest_dir.join("build.rs"));
        rerun(&mut lines, &self.manifest_dir.join("Cargo.toml"));
        let mut report = ScanReport { crate_name: self.crate_name.clone(), ..ScanReport::default() };

        let started = Instant::now();
        let (models, dependency_files) = match ModelSet::read(&self.dependencies) {
            Ok(read) => read,
            Err(error) => return Outcome { lines, errors: vec![format!("the .xamlmeta of a dependency cannot be read: {error}")] },
        };
        report.read_dependencies = started.elapsed();
        report.dependency_files = dependency_files.len();
        report.dependency_models = models.len();
        for file in &dependency_files {
            rerun(&mut lines, file);
        }

        let root = match source_root(&self.manifest_dir, self.source_root.as_deref()) {
            Ok(root) => root,
            Err(error) => return Outcome { lines, errors: vec![error] },
        };
        let started = Instant::now();
        let scan = scan_crate(&ScanOptions::new(&self.crate_name, root).with_dependencies(models));
        report.scanned(&scan, started.elapsed());
        for file in &scan.files {
            rerun(&mut lines, &file.path);
        }
        // What the scanner cannot read of a declaration does not fail the build of the
        // crate that declares it: the compiler of a document that names the type reports it.
        for diagnostic in scan.diagnostics_of(Severity::Error) {
            lines.push(format!("cargo::warning={diagnostic}"));
        }

        let mut model = scan.model;
        if model.name.is_empty() {
            // A crate that states no assembly is the assembly named after the crate, as
            // the type systems name it.
            model.name = self.crate_name.clone();
        }
        model.crate_name = self.crate_name.clone();
        model.dependencies = self.dependencies.iter().map(|path| path.display().to_string()).collect();
        for path in &self.compiled_markup {
            rerun(&mut lines, path);
            let read = fs::read_to_string(path).map_err(|error| error.to_string()).and_then(|text| XamlMetadata::parse(&text));
            match read {
                Ok(read) if read.name == model.name => model.documents.extend(read.documents),
                Ok(read) => {
                    let error = format!("{}: the file describes the assembly `{}`, the crate is `{}`", path.display(), read.name, model.name);
                    return Outcome { lines, errors: vec![error] };
                }
                Err(error) => return Outcome { lines, errors: vec![format!("{}: {error}", path.display())] },
            }
        }
        let text = model.to_json();
        let path = self.out_dir.join(format!("{}.xamlmeta", self.crate_name));
        let started = Instant::now();
        report.model_bytes = text.len();
        match write_if_changed(&path, &text) {
            Ok(written) => report.model_written = written,
            Err(error) => return Outcome { lines, errors: vec![error] },
        }
        report.write = started.elapsed();
        if let Err(error) = report.write(&self.out_dir) {
            return Outcome { lines, errors: vec![error] };
        }
        lines.push(format!("cargo::metadata=xamlmeta={}", path.display()));
        Outcome { lines, errors: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AssemblyModel;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures")
    }

    /// Not from upstream: the export scans the sources of the crate, writes its model
    /// with the paths of the files of its dependencies, names every file it read and
    /// nothing else, and writes what it cost; a second run over the same sources writes
    /// the same text, that is, nothing.
    #[test]
    fn export_writes_the_model_once_and_names_its_inputs() {
        let out = env::temp_dir().join(format!("ferroui-build-scan-export-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let directory = fixtures().join("scanner");
        let export = || Export::new(directory.clone(), out.join("fixture"), "fixture", Vec::new()).source_root("lib.rs").execute();
        let first = export();
        assert_eq!(first.errors, Vec::<String>::new());
        let file = out.join("fixture").join("fixture.xamlmeta");
        assert_eq!(first.lines.last(), Some(&format!("cargo::metadata=xamlmeta={}", file.display())));
        assert!(first.lines.contains(&format!("cargo::rerun-if-changed={}", directory.join("controls").join("border.rs").display())), "{:?}", first.lines);
        // What the scanner does not read is a warning of the build.
        assert!(first.lines.iter().any(|line| line.starts_with("cargo::warning=") && line.contains("error FRN9010")), "{:?}", first.lines);
        // Every other line names an input: the script runs again for nothing else.
        let inputs: Vec<&String> = first.lines.iter().filter(|line| line.starts_with("cargo::rerun-if-changed=")).collect();
        let warnings = first.lines.iter().filter(|line| line.starts_with("cargo::warning=")).count();
        assert_eq!(inputs.len() + warnings + 1, first.lines.len(), "{:?}", first.lines);
        assert!(inputs.iter().all(|line| line.ends_with(".rs") || line.ends_with("Cargo.toml")), "{inputs:?}");

        let text = fs::read_to_string(&file).expect("the .xamlmeta of the fixture");
        let model = AssemblyModel::parse(&text).expect("a file of format 2");
        assert_eq!((model.name.as_str(), model.crate_name.as_str()), ("Fixture", "fixture"));
        assert!(model.find_type("Fixture.Controls.Border").is_some() && model.documents.is_empty() && model.dependencies.is_empty());
        let report = fs::read_to_string(out.join("fixture").join(SCAN_REPORT_FILE)).expect("the report of the scan");
        assert!(report.starts_with("crate: fixture\nmodels of the dependencies: 0 read from 0 files in "), "{report}");
        assert!(report.contains(&format!("\nsource files scanned: {}\n", inputs.len() - 2)), "{report}");
        assert!(report.contains(&format!("\nmodel: {} bytes, written in ", text.len())), "{report}");

        let second = export();
        assert_eq!((second.errors, &second.lines), (Vec::new(), &first.lines));
        assert_eq!(fs::read_to_string(&file).ok(), Some(text.clone()));
        let report = fs::read_to_string(out.join("fixture").join(SCAN_REPORT_FILE)).expect("the report of the scan");
        assert!(report.contains(&format!("\nmodel: {} bytes, unchanged, not written in ", text.len())), "{report}");

        // A crate built on it reads the file, names it as an input and writes its path.
        let dependent = Export::new(fixtures().join("dependent"), out.join("dependent"), "dependent", vec![file.clone()]).execute();
        assert_eq!(dependent.errors, Vec::<String>::new());
        assert!(dependent.lines.contains(&format!("cargo::rerun-if-changed={}", file.display())), "{:?}", dependent.lines);
        let text = fs::read_to_string(out.join("dependent").join("dependent.xamlmeta")).expect("the .xamlmeta of the dependent crate");
        let model = AssemblyModel::parse(&text).expect("a file of format 2");
        assert_eq!((model.name.as_str(), model.crate_name.as_str()), ("dependent", "dependent"));
        assert_eq!(model.dependencies, [file.display().to_string()]);
        assert_eq!(model.find_type("Card").and_then(|card| card.base.as_ref()).map(|base| base.text.as_str()), Some("::fixture::controls::border::Border"));

        // The documents of a checked-in file of the crate are exported with its model; a
        // file of another assembly is an error.
        let checked_in = out.join("compiled_xaml.xamlmeta");
        let document = crate::xaml_metadata::DocumentModel {
            uri: "ferres://dependent/Card.xaml".to_string(),
            root_type: "Card".to_string(),
            class_rust_path: Some("::dependent::Card".to_string()),
            build_path: None,
            populate_path: Some("::dependent::compiled_xaml::populate".to_string()),
            public: true,
        };
        let mut metadata = XamlMetadata { name: "dependent".to_string(), crate_name: "dependent".to_string(), documents: vec![document.clone()], dependencies: Vec::new() };
        fs::write(&checked_in, metadata.to_json()).expect("the checked-in file");
        let with_documents = || {
            Export::new(fixtures().join("dependent"), out.join("documents"), "dependent", vec![file.clone()])
                .compiled_markup(&checked_in.display().to_string())
                .execute()
        };
        let exported = with_documents();
        assert_eq!(exported.errors, Vec::<String>::new());
        assert!(exported.lines.contains(&format!("cargo::rerun-if-changed={}", checked_in.display())), "{:?}", exported.lines);
        let text = fs::read_to_string(out.join("documents").join("dependent.xamlmeta")).expect("the .xamlmeta with the documents");
        let model = AssemblyModel::parse(&text).expect("a file of format 2");
        assert_eq!(model.documents, [document]);
        assert!(model.find_type("Card").is_some());
        assert_eq!(XamlMetadata::parse(&text).map(|read| read.documents), Ok(model.documents.clone()));
        metadata.name = "Other".to_string();
        fs::write(&checked_in, metadata.to_json()).expect("the checked-in file");
        let other = with_documents();
        assert_eq!(other.errors.len(), 1, "{:?}", other.errors);
        assert!(other.errors[0].ends_with("the file describes the assembly `Other`, the crate is `dependent`"), "{:?}", other.errors);

        let missing = Export::new(fixtures().join("dependent"), out.join("missing"), "dependent", Vec::new()).source_root("main.rs").execute();
        assert_eq!(missing.errors.len(), 1, "{:?}", missing.errors);
        assert!(missing.errors[0].ends_with("main.rs, is not a file"), "{:?}", missing.errors);
        let _ = fs::remove_dir_all(&out);
    }
}
