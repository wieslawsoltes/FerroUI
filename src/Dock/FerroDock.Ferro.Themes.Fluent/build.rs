//! Compiles the source theme as a class document with all original resources.
use std::path::{Path, PathBuf};
fn documents(root: &Path, path: &Path, result: &mut Vec<(String, String)>) {
    println!("cargo::rerun-if-changed={}", path.display());
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .expect("theme directory")
        .map(|entry| entry.expect("theme entry").path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            documents(root, &entry, result);
        } else if entry
            .extension()
            .is_some_and(|extension| extension == "xaml")
        {
            println!("cargo::rerun-if-changed={}", entry.display());
            result.push((
                entry
                    .strip_prefix(root)
                    .expect("theme path")
                    .components()
                    .map(|component| component.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
                std::fs::read_to_string(entry).expect("theme document"),
            ));
        }
    }
}
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let mut source = Vec::new();
    documents(&root, &root, &mut source);
    let source: Vec<(&str, &str)> = source
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let presets: Vec<(&str, &str)> = source
        .iter()
        .copied()
        .filter(|(name, _)| name.starts_with("Presets/"))
        .collect();
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .default_compile_bindings(true)
        .embed_assets(&["Assets"])
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_xaml")
                .documents(&source)
                .class_document("DockFluentTheme.xaml")
                .constructor(ferroui_build::ClassConstructor::ServiceProvider(
                    "with_service_provider",
                )),
        )
        .compile_group(ferroui_build::XamlGroup::new("compiled_presets").documents(&presets))
        .run();
}
