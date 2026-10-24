//! Compiles the markup documents of the crate (every `*.xaml` under the crate directory, each as
//! a group of its own: a document with a class as the document of its class, with the documents
//! of the crate it includes) against the type models, embeds the assets (`Assets/`), and exports
//! the type model of the crate (docs/porting/xaml.md, 9.5.13 and 9.6).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_build::{Build, TypeSystem, XamlGroup};

const ASSEMBLY: &str = "FerroUI.Diagnostics";
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests"];

fn collect(root: &Path, directory: &Path, found: &mut Vec<(String, String)>) {
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
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            found.push((parts.join("/"), text));
        }
    }
}

/// The module name of the compiled markup of a document.
fn module_name(path: &str) -> String {
    let stem = path.trim_end_matches(".xaml");
    let mut name = String::from("compiled_");
    let mut previous_lower = false;
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && previous_lower {
                name.push('_');
            }
            name.push(c.to_ascii_lowercase());
            previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            name.push('_');
            previous_lower = false;
        }
    }
    name
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let mut documents = Vec::new();
    collect(&root, &root, &mut documents);
    let all: Vec<(&str, &str)> = documents.iter().map(|(path, text)| (path.as_str(), text.as_str())).collect();

    let mut build = Build::from_env()
        .type_system(TypeSystem::Model)
        // The upstream project is built with compiled bindings as the default of its documents.
        .default_compile_bindings(true)
        .embed_assets(&["Assets"]);
    for (path, text) in &all {
        // A document that includes another one of the crate (by its absolute URI, or by a path
        // rooted in the crate, `Source="/Assets/Icons.xaml"`) is given every document, of which
        // the compiler takes the ones it includes.
        let includes = text.contains(&format!("ferres://{ASSEMBLY}/")) || text.contains("Source=\"/");
        let has_class = text.contains("x:Class=");
        let group = XamlGroup::new(&module_name(path));
        let group = if includes { group.documents(&all) } else { group.documents(&[(*path, *text)]) };
        let group = if has_class { group.class_document(path) } else { group };
        build = build.compile_group(group);
    }
    build.run();
}
