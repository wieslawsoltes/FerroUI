//! The diagnostics of a build (`ferroui_build::Build` against the type models) for real
//! documents: a warning of the transform with its code, its document and its position,
//! the severities of an EditorConfig file, warnings as errors, a group that does not
//! transform and a document the emitter refuses. Not from upstream, whose build task has
//! no tests.
//!
//! The build is the one of a build script, run in the test: the crate it builds is a
//! directory with an empty root file, and its dependencies are the `.xamlmeta` files of
//! the framework crates, written here from the scan of their sources as their build
//! scripts export them.

use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::{Build, Outcome, TypeSystem, XamlGroup};

use super::model_transform::scanned_models;

const NAMESPACES: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

/// An item container inside an item template: upstream's warning `ItemContainerInsideTemplate`.
fn list() -> String {
    format!("<ListBox {NAMESPACES}>\n    <ListBox.ItemTemplate>\n        <DataTemplate>\n            <ListBoxItem />\n        </DataTemplate>\n    </ListBox.ItemTemplate>\n</ListBox>")
}

fn border() -> String {
    format!("<Border {NAMESPACES}/>")
}

/// The directory of the test, with the crate that is built and the `.xamlmeta` files of
/// the crates it depends on.
struct Fixture {
    root: PathBuf,
    dependencies: Vec<PathBuf>,
}

impl Fixture {
    fn new() -> Self {
        // A directory of its own for each fixture: the tests of the file run at the same time.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("ferroui-build-diagnostics-{}-{number}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crate")).expect("the directory of the crate");
        fs::write(root.join("crate").join("lib.rs"), "//! A crate without types.\n").expect("the root file of the crate");
        // The base crate, the controls and the XAML runtime library; the model of the test
        // crate is not a dependency of the crate that is built.
        let mut dependencies = Vec::new();
        for model in scanned_models().into_iter().take(3) {
            let path = root.join(format!("{}.xamlmeta", model.crate_name));
            fs::write(&path, model.to_json()).expect("the .xamlmeta of a dependency");
            dependencies.push(path);
        }
        Self { root, dependencies }
    }

    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("crate").join(name);
        fs::write(&path, text).expect("a file of the crate");
        path
    }

    /// The build `name` of `documents` as one group, with what `configure` adds.
    fn build(&self, name: &str, documents: &[(&str, &str)], configure: impl FnOnce(Build) -> Build) -> Outcome {
        let build = Build::new(self.root.join("crate"), self.root.join("out").join(name), "diagnostics_fixture", self.dependencies.clone())
            .source_root("lib.rs")
            .type_system(TypeSystem::Model)
            .compile_group(XamlGroup::new("compiled_xaml").documents(documents));
        configure(build).execute()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn warnings(outcome: &Outcome) -> Vec<&str> {
    outcome.lines.iter().filter_map(|line| line.strip_prefix("cargo::warning=")).filter(|line| line.contains(": warning FRN")).collect()
}

fn rerun_if_changed(outcome: &Outcome, path: &Path) -> bool {
    outcome.lines.contains(&format!("cargo::rerun-if-changed={}", path.display()))
}

#[test]
fn build_reports_the_diagnostics_of_the_compiler_with_their_codes() {
    let fixture = Fixture::new();
    let (list, border) = (list(), border());
    let documents = [("Plain.xaml", border.as_str()), ("Views/List.xaml", list.as_str())];

    // A warning of the transform: the code, the document and the position; the build succeeds.
    let plain = fixture.build("plain", &documents, |build| build);
    assert_eq!(plain.errors, Vec::<String>::new());
    let found = warnings(&plain);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("Views/List.xaml(4,14): warning FRN2208: "), "{found:?}");

    // Warnings as errors: the same diagnostic fails the build, and it is the only error.
    let strict = fixture.build("strict", &documents, |build| build.warnings_as_errors(true));
    assert!(warnings(&strict).is_empty(), "{:?}", strict.lines);
    assert_eq!(strict.errors.len(), 1, "{:?}", strict.errors);
    assert!(strict.errors[0].starts_with("Views/List.xaml(4,14): error FRN2208: "), "{:?}", strict.errors);

    // An EditorConfig file silences the code, also under warnings as errors; the file is an
    // input of the build.
    let silent = fixture.file("silent.editorconfig", "[*.xaml]\nferro_xaml_diagnostic.FRN2208.severity = none\n");
    let silenced = fixture.build("silenced", &documents, |build| build.analyzer_config_files(&["silent.editorconfig"]).warnings_as_errors(true));
    assert_eq!(silenced.errors, Vec::<String>::new());
    assert!(warnings(&silenced).is_empty(), "{:?}", silenced.lines);
    assert!(rerun_if_changed(&silenced, &silent), "{:?}", silenced.lines);

    // An EditorConfig file makes the code an error: the transform fails with it, and the
    // build reports the diagnostic and not a second error for the group.
    fixture.file("error.editorconfig", "ferro_xaml_diagnostic.FRN2208.severity = error\n");
    let raised = fixture.build("raised", &documents, |build| build.analyzer_config_files(&["error.editorconfig", "missing.editorconfig"]));
    assert_eq!(raised.errors.len(), 1, "{:?}", raised.errors);
    assert!(raised.errors[0].starts_with("Views/List.xaml(4,14): error FRN2208: "), "{:?}", raised.errors);

    // A group that does not transform: one error with the code of a transform error, not
    // one for each of its documents.
    let missing = format!("<Missing {NAMESPACES}/>");
    let broken = fixture.build("broken", &[("Plain.xaml", border.as_str()), ("Broken.xaml", missing.as_str())], |build| build);
    assert_eq!(broken.errors.len(), 1, "{:?}", broken.errors);
    assert!(broken.errors[0].contains("error FRN2000: "), "{:?}", broken.errors);
    assert!(broken.errors[0].contains("Missing"), "{:?}", broken.errors);

    // A document the emitter refuses: the code of an emit error, with the document.
    let refused = fixture.build("refused", &[("a-b.xaml", border.as_str()), ("a_b.xaml", border.as_str())], |build| build);
    assert_eq!(
        refused.errors,
        ["a_b.xaml: error FRN3000: the generated function `build_a_b_xaml` would also be defined for the document `a-b.xaml`; rename one of them"]
    );
}

/// A method named as the handler of an event that no method of the root object answers to
/// is one error of the build: the code of the compiler's diagnostic, the document, the
/// place of the attribute, the method and the event.
#[test]
fn build_reports_a_handler_that_is_not_found() {
    let fixture = Fixture::new();
    let border = border();
    let button = format!("<StackPanel {NAMESPACES}>\n    <Button Content='Go'\n            Click='OnGo'/>\n</StackPanel>");
    let outcome = fixture.build("handler", &[("Plain.xaml", border.as_str()), ("Views/Buttons.xaml", button.as_str())], |build| build);
    assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
    assert!(
        outcome.errors[0].starts_with(
            "Views/Buttons.xaml(3,13): error FRN3000: No method `OnGo` for `Click` (System.EventHandler`1[FerroUI.Interactivity.RoutedEventArgs]): \
             the document has no class (`x:Class`), and the type of its root object has no method of that name that the delegate can call."
        ),
        "{:?}",
        outcome.errors
    );
}
