//! A measure of the ControlCatalog (`samples/ControlCatalog`) against the build-time type
//! system: what the build of the sample, which compiles its documents, emits and refuses.
//! Nothing is written into the sample: the build is run here, as its build script runs it
//! (`ferroui_build::Build` with `TypeSystem::Model`), over the files of the sample as they
//! are (without the placeholder artwork its build puts in), into a temporary directory.
//!
//! The type models are the ones the build scripts of the crates would export, written
//! here with the export of those scripts (`ferroui_build::export::Export`): the base
//! crate, the controls, the XAML runtime library, the dialogs and the two themes (with
//! their compiled documents), the OpenGL controls and the view model library of the
//! samples. The colour picker is built as its build script builds it (its theme documents
//! compiled as one group), so that its model has the compiled documents `App.xaml`
//! includes. The model of the sample is the scan of its sources.
//!
//! Every document is compiled as a group of its own (a document with a class as the
//! document of its class, with the documents of the sample it includes), so that a
//! document that is refused does not take another one with it. The test prints, and
//! `docs/porting/xaml-compiler/HANDOVER.md` (section 18) records: how many documents
//! compile, how many are refused and why, by reason with counts, and the size of the
//! emitted Rust.
//!
//! The first measure was taken twice, the second time with four registrations of the
//! colour picker counted as read: the scanner did not read the registrations a macro of
//! that crate makes for each type it is invoked with, and with a cast of a crate not read
//! the models answer no question about a cast. The scanner reads them now (a rule of a
//! macro that repeats is expanded for the registrations of a function), so the measure is
//! taken once, and the test fails if a model has a cast the scan did not read.
//!
//! "Compiled" here is what the build emits. That rustc compiles it and that it loads to
//! the tree of the run-time loader is proven by the sample itself, whose build compiles
//! every document but the ones its list names (`build/compiled_documents.rs`, `REFUSED`)
//! and whose tests compare every compiled class with the run-time loader's
//! (`tests/compiled_markup.rs`); the measure fails when the documents it finds refused are
//! not exactly that list, so the two cannot drift apart.
//!
//! ```text
//! cargo test -p ferroui-markup-xaml-tests --lib emitter::catalog_measure -- --ignored --nocapture
//! ```
//!
//! Not from upstream.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::export::Export;
use ferroui_build::model::AssemblyModel;
use ferroui_build::scanner::{scan_crate, ScanOptions, Severity};
use ferroui_build::{Build, TypeSystem, XamlGroup};

/// The lists of the build of the sample (`samples/ControlCatalog/build.rs`).
#[allow(dead_code)]
#[path = "../../../samples/ControlCatalog/build/compiled_documents.rs"]
mod compiled_documents;

/// How a question the type models cannot answer starts in the reason of a refusal.
const CANNOT_ANSWER: &str = "the type system of the host cannot answer: ";

/// Every `.xaml` file below `directory`, by its path below `root` with `/` separators.
fn documents_below(root: &Path, directory: &Path, found: &mut Vec<(String, String)>) {
    let mut entries: Vec<PathBuf> =
        fs::read_dir(directory).unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display())).map(|entry| entry.expect("an entry").path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" {
                documents_below(root, &path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "xaml") {
            let relative = path.strip_prefix(root).expect("a path below the sample");
            let parts: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            found.push((parts.join("/"), text));
        }
    }
}

/// The reason of an error of the build as a kind: without the place and without the names
/// that differ from one document to the next (the names in quotes and in backticks, the
/// class of a `class:` assignment, the line and the position), so that documents refused
/// for the same thing are counted together. A question the type models cannot answer is
/// counted by its first question.
fn kind_of(reason: &str) -> String {
    let reason = match reason.split_once(CANNOT_ANSWER) {
        Some((start, questions)) => format!("{start}{CANNOT_ANSWER}{}", questions.split("; ").next().unwrap_or(questions)),
        None => reason.to_string(),
    };
    let characters: Vec<char> = reason.chars().collect();
    let mut kind = String::with_capacity(reason.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let quoted = matches!(character, '`' | '\'' | '"')
            .then(|| characters[index + 1..].iter().position(|next| *next == character))
            .flatten()
            // A name, not the arity of a generic type (`List`1`).
            .filter(|length| *length > 0 && !characters[index + 1].is_ascii_digit());
        if let Some(length) = quoted {
            kind.push(character);
            kind.push('_');
            kind.push(character);
            index += length + 2;
            continue;
        }
        kind.push(character);
        index += 1;
    }
    // `(line 12 position 34)`, `Line 12, position 34.` and `class:name:`.
    let mut words: Vec<String> = Vec::new();
    for word in kind.split(' ') {
        let numbered = words.last().is_some_and(|last| matches!(last.trim_start_matches('(').to_lowercase().as_str(), "line" | "position"));
        let digits = word.trim_end_matches([')', ',', '.']);
        if numbered && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            words.push(word.replace(digits, "N"));
        } else if let Some(rest) = word.strip_prefix("class:") {
            words.push(format!("class:_{}", if rest.ends_with(':') { ":" } else { "" }));
        } else {
            words.push(word.to_string());
        }
    }
    words.join(" ")
}

/// The registrations with the untyped value conversions the scan of a crate did not read,
/// by kind with counts.
fn unread_of(model: &AssemblyModel) -> String {
    match model.unread_value_types.is_empty() {
        true => "none".to_string(),
        false => model.unread_value_types.iter().map(|(registration, count)| format!("{count} register_{registration}")).collect::<Vec<_>>().join(", "),
    }
}

/// What a build of the documents did.
struct Measure {
    /// The documents that compiled.
    compiled: Vec<String>,
    /// The size of the emitted Rust of the compiled documents.
    bytes: usize,
    lines: usize,
    /// The refused documents by the kind of their first error.
    kinds: BTreeMap<String, Vec<String>>,
    /// The first errors of the refused documents as the build reported them, by kind.
    reasons: BTreeMap<String, Vec<String>>,
    /// The questions the type models cannot answer, each with the number of documents that
    /// ask it.
    questions: BTreeMap<String, usize>,
    errors: usize,
    seconds: f64,
}

/// Builds the documents of the sample against the models `dependencies`, each as a group of
/// its own, into `out_dir`.
fn measure(sample: &Path, out_dir: &Path, dependencies: Vec<PathBuf>, documents: &[(String, String)]) -> Measure {
    let borrowed: Vec<(&str, &str)> = documents.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    let mut build = Build::new(sample.to_path_buf(), out_dir.to_path_buf(), "control-catalog", dependencies)
        .type_system(TypeSystem::Model)
        // The sample loads its documents with compiled bindings as their default.
        .default_compile_bindings(true);
    let module_of = |index: usize| format!("document_{index}");
    for (index, (name, text)) in documents.iter().enumerate() {
        // A document that includes another one of the sample is given every document, of
        // which the compiler takes the ones it includes.
        let includes = text.contains("ferres://ControlCatalog/");
        let group = XamlGroup::new(&module_of(index));
        let group = match (text.contains("x:Class="), includes) {
            (true, true) => group.documents(&borrowed).class_document(name),
            (true, false) => group.documents(&[(name.as_str(), text.as_str())]).class_document(name),
            (false, _) => group.documents(&[(name.as_str(), text.as_str())]),
        };
        build = build.compile_group(group);
    }
    let started = std::time::Instant::now();
    let outcome = build.execute();
    let seconds = started.elapsed().as_secs_f64();
    let written = |index: usize| fs::read_to_string(out_dir.join("xaml").join(format!("{}.rs", module_of(index)))).ok();

    // The errors, by the document they name first (the longest name that fits).
    let mut reported: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut unplaced: Vec<String> = Vec::new();
    for error in &outcome.errors {
        let document = documents
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| error.strip_prefix(*name).is_some_and(|rest| rest.starts_with('(') || rest.starts_with(": ")))
            .max_by_key(|name| name.len());
        match document {
            Some(document) => reported.entry(document).or_default().push(error.clone()),
            None => unplaced.push(error.clone()),
        }
    }
    // An error without a document (a diagnostic of a group transformer) is the error of
    // the one document that wrote no file and reported nothing, when there is one.
    let silent: Vec<&str> = documents
        .iter()
        .enumerate()
        .filter(|(index, (name, _))| !reported.contains_key(name.as_str()) && written(*index).is_none())
        .map(|(_, (name, _))| name.as_str())
        .collect();
    if let ([document], false) = (silent.as_slice(), unplaced.is_empty()) {
        reported.entry(document).or_default().append(&mut unplaced);
    }
    for error in &unplaced {
        println!("an error that names no document: {error}");
    }

    let mut questions: BTreeMap<String, usize> = BTreeMap::new();
    for errors in reported.values() {
        let mut asked: Vec<&str> =
            errors.iter().filter_map(|error| error.split_once(CANNOT_ANSWER)).flat_map(|(_, questions)| questions.split("; ")).collect();
        asked.sort();
        asked.dedup();
        for question in asked {
            *questions.entry(question.to_string()).or_default() += 1;
        }
    }

    // A document compiled when the file of its group was written and the build reported
    // nothing for it.
    let mut result = Measure {
        compiled: Vec::new(),
        bytes: 0,
        lines: 0,
        kinds: BTreeMap::new(),
        reasons: BTreeMap::new(),
        questions,
        errors: outcome.errors.len(),
        seconds,
    };
    for (index, (name, _)) in documents.iter().enumerate() {
        match (reported.get(name.as_str()), written(index)) {
            (None, Some(text)) => {
                result.compiled.push(name.clone());
                result.bytes += text.len();
                result.lines += text.lines().count();
            }
            (None, None) => result.kinds.entry("(the group of the document wrote no file and reported nothing)".to_string()).or_default().push(name.clone()),
            (Some(errors), _) => {
                // The first error says why; the place and the document are left out.
                let first = &errors[0];
                let reason = first.strip_prefix(name.as_str()).unwrap_or(first);
                let reason = reason.strip_prefix("{unknown document}").unwrap_or(reason);
                let reason = match reason.strip_prefix('(').and_then(|rest| rest.split_once("): ")) {
                    Some((_, rest)) => rest,
                    None => reason.strip_prefix(": ").unwrap_or(reason),
                };
                // The text of the compiler that follows the description of a missing method.
                let reason = reason.split(". Unable to find suitable setter").next().unwrap_or(reason);
                let kind = kind_of(reason);
                if !reason.contains(CANNOT_ANSWER) {
                    result.reasons.entry(kind.clone()).or_default().push(format!("{name}: {reason}"));
                }
                result.kinds.entry(kind).or_default().push(name.clone());
            }
        }
    }
    assert_eq!(result.compiled.len() + result.kinds.values().map(Vec::len).sum::<usize>(), documents.len());
    result
}

fn print(title: &str, documents: usize, measure: &Measure) {
    println!("==== {title} ====");
    println!("the build ran for {:.1} s and reported {} errors", measure.seconds, measure.errors);
    println!("documents: {documents}");
    println!("compiled:  {}", measure.compiled.len());
    println!("refused:   {}", documents - measure.compiled.len());
    println!("emitted Rust of the compiled documents: {} bytes, {} lines", measure.bytes, measure.lines);
    let mut by_count: Vec<(&String, &Vec<String>)> = measure.kinds.iter().collect();
    by_count.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    println!("---- refused, by reason ({} reasons) ----", by_count.len());
    for (kind, names) in &by_count {
        println!("{:>4}  {kind}", names.len());
        println!("      {}", names.iter().take(4).map(String::as_str).collect::<Vec<_>>().join(", "));
    }
    let mut asked: Vec<(&String, &usize)> = measure.questions.iter().collect();
    asked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("---- the questions the type models cannot answer, by the number of documents that ask ({}) ----", asked.len());
    for (question, count) in &asked {
        println!("{count:>4}  {question}");
    }
    println!("---- the first error of each refused document, for the reasons of at most eight documents ----");
    for (kind, reasons) in measure.reasons.iter().filter(|(_, reasons)| reasons.len() <= 8) {
        println!("{kind}");
        for reason in reasons {
            println!("      {reason}");
        }
    }
    println!("---- compiled ({}) ----", measure.compiled.len());
    println!("{}", measure.compiled.join(", "));
}

/// Not from upstream: the ControlCatalog compiled against the type models, measured.
#[test]
#[ignore = "a measurement: scans ten crates and compiles the documents of the ControlCatalog; run it with --nocapture"]
fn measure_the_control_catalog_against_the_models() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let out = std::env::temp_dir().join(format!("ferroui-catalog-measure-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);

    // The type models of the crates the sample is built on, as their build scripts export them.
    let models: std::cell::RefCell<Vec<AssemblyModel>> = std::cell::RefCell::new(Vec::new());
    let export = |directory: &str, package: &str, dependencies: &[&PathBuf], compiled_markup: Option<&str>| -> PathBuf {
        let out_dir = out.join(package);
        fs::create_dir_all(&out_dir).expect("the directory of the model");
        let export = Export::new(repository.join(directory), out_dir.clone(), package, dependencies.iter().map(|path| (*path).clone()).collect());
        let export = match compiled_markup {
            Some(path) => export.compiled_markup(path),
            None => export,
        };
        let outcome = export.execute();
        assert!(outcome.errors.is_empty(), "the model of {package} is not exported: {:?}", outcome.errors);
        let not_read = outcome.lines.iter().filter(|line| line.starts_with("cargo::warning=")).count();
        let path = out_dir.join(format!("{}.xamlmeta", package.replace('-', "_")));
        let model = fs::read_to_string(&path).map_err(|error| error.to_string()).and_then(|text| AssemblyModel::parse(&text));
        let model = model.unwrap_or_else(|error| panic!("the model of {package} cannot be read: {error}"));
        println!(
            "the model of {package}: {} types, {} compiled documents; {not_read} declarations the scanner does not read; registrations with the untyped value conversions not read: {}",
            model.types.len(),
            model.documents.len(),
            unread_of(&model)
        );
        models.borrow_mut().push(model);
        path
    };
    let base = export("src/FerroUI.Base", "ferroui-base", &[], None);
    let controls = export("src/FerroUI.Controls", "ferroui-controls", &[&base], None);
    let markup_xaml = export("src/Markup/FerroUI.Markup.Xaml", "ferroui-markup-xaml", &[&base, &controls], None);
    let dialogs = export("src/FerroUI.Dialogs", "ferroui-dialogs", &[&base, &controls, &markup_xaml], None);
    let simple = export(
        "src/FerroUI.Themes.Simple",
        "ferroui-themes-simple",
        &[&base, &controls, &dialogs, &markup_xaml],
        Some("compiled_xaml.xamlmeta"),
    );
    let fluent = export(
        "src/FerroUI.Themes.Fluent",
        "ferroui-themes-fluent",
        &[&base, &controls, &dialogs, &markup_xaml],
        Some("compiled_xaml.xamlmeta"),
    );
    // The colour picker, as its build script builds it: its theme documents compiled as one
    // group, with compiled bindings as their default.
    let color_picker = {
        let directory = repository.join("src/FerroUI.Controls.ColorPicker");
        let mut documents = Vec::new();
        documents_below(&directory, &directory.join("Themes"), &mut documents);
        let borrowed: Vec<(&str, &str)> = documents.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
        let out_dir = out.join("ferroui-controls-color-picker");
        fs::create_dir_all(&out_dir).expect("the directory of the model");
        let outcome = Build::new(directory, out_dir.clone(), "ferroui-controls-color-picker", vec![base.clone(), controls.clone(), markup_xaml.clone()])
            .type_system(TypeSystem::Model)
            .default_compile_bindings(true)
            .compile_group(XamlGroup::new("compiled_xaml").documents(&borrowed))
            .execute();
        assert!(outcome.errors.is_empty(), "the colour picker is not built: {:?}", outcome.errors);
        let path = out_dir.join("ferroui_controls_color_picker.xamlmeta");
        let model = fs::read_to_string(&path).map_err(|error| error.to_string()).and_then(|text| AssemblyModel::parse(&text));
        let model = model.unwrap_or_else(|error| panic!("the model of the colour picker cannot be read: {error}"));
        println!(
            "the model of ferroui-controls-color-picker: {} types, {} compiled documents (of {} theme documents); registrations with the untyped value conversions not read: {}",
            model.types.len(),
            model.documents.len(),
            documents.len(),
            unread_of(&model)
        );
        assert_eq!(model.documents.len(), documents.len(), "every theme document of the colour picker is compiled");
        models.borrow_mut().push(model);
        path
    };
    let open_gl = export("src/FerroUI.OpenGL", "ferroui-opengl", &[&base, &controls], None);
    let mini_mvvm = export("samples/MiniMvvm", "mini-mvvm", &[&base], None);

    // The model of the sample, as the build scans it.
    let sample = repository.join("samples").join("ControlCatalog");
    let scan = scan_crate(&ScanOptions::new("control_catalog", sample.join("lib.rs")).with_dependencies(models.borrow().clone()));
    println!("==== the scan of the ControlCatalog ====\n{}", scan.summary());
    let not_read: Vec<String> = scan.diagnostics_of(Severity::Error).map(|diagnostic| diagnostic.to_string()).collect();
    println!("declarations the scanner does not read: {}", not_read.len());
    for diagnostic in not_read.iter().take(40) {
        println!("    {diagnostic}");
    }
    println!("registrations with the untyped value conversions not read: {}", unread_of(&scan.model));

    let mut documents = Vec::new();
    documents_below(&sample, &sample, &mut documents);
    let with_class = documents.iter().filter(|(_, text)| text.contains("x:Class=")).count();
    println!("the ControlCatalog: {} documents, {with_class} with a class", documents.len());

    let dependencies = vec![base, controls, markup_xaml, dialogs, simple, fluent, color_picker, open_gl, mini_mvvm];
    // No model has a cast its scan did not read: no question about a cast is refused.
    for model in models.borrow().iter().chain(std::iter::once(&scan.model)) {
        let casts: Vec<&(String, i64)> = model.unread_value_types.iter().filter(|(registration, _)| registration == "cast").collect();
        assert!(casts.is_empty(), "{}: casts the scan did not read: {casts:?}", model.crate_name);
    }
    let measured = measure(&sample, &out.join("control-catalog"), dependencies, &documents);
    print("the ControlCatalog against the type models", documents.len(), &measured);

    let _ = fs::remove_dir_all(&out);
    assert!(measured.questions.is_empty(), "the type models cannot answer: {:?}", measured.questions);
    // The documents the sample leaves out of its build are the refused ones, no more and
    // no fewer: every other document is compiled by rustc and compared there.
    let refused: std::collections::BTreeSet<&str> = measured.kinds.values().flatten().map(String::as_str).collect();
    let listed: std::collections::BTreeSet<&str> = compiled_documents::REFUSED.iter().map(|(name, _)| *name).collect();
    assert_eq!(refused, listed, "the refused documents and the list of the sample (samples/ControlCatalog/build/compiled_documents.rs, REFUSED) differ");
}
