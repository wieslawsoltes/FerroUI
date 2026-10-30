//! Reads the same locked published command source used by the original Rust model.
use super::*;
use ferroui_build_scan::{
    model::{RustType, TypeKind, TypeModel},
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};
use std::path::PathBuf;

pub(super) struct Command {
    pub scan: Scan,
    pub metadata: TypeModel,
    pub source: native_adapter::RustSourceDeclaration,
    pub version: String,
}
// Cargo writes normalized string entries in each generated lock-file package block.
fn entry(block: &str, key: &str) -> Option<String> {
    block.lines().find_map(|line| {
        let (name, value) = line.split_once(" = ")?;
        if name != key {
            return None;
        }
        Some(value.strip_prefix('"')?.strip_suffix('"')?.to_owned())
    })
}
pub(super) fn read(workspace: &std::path::Path) -> Command {
    let lock = std::fs::read_to_string(workspace.join("Cargo.lock")).unwrap();
    let mut packages = lock
        .split("[[package]]")
        .filter(|p| entry(p, "name").as_deref() == Some("ferroui-reactiveui"));
    let package = packages.next().expect("Locked ReactiveUI package");
    assert!(
        packages.next().is_none(),
        "Command source requires one resolved ReactiveUI version"
    );
    let version = entry(package, "version").unwrap();
    // A member of the workspace (no source in the lock file) is read where it is.
    let source = entry(package, "source");
    if source.is_none() {
        return scan_command(&workspace.join("src/FerroUI.ReactiveUI"), version);
    }
    assert_eq!(
        source.as_deref(),
        Some("registry+https://github.com/rust-lang/crates.io-index"),
        "Resolve non-registry command sources explicitly"
    );
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .expect("Cargo home directory"),
            )
            .join(".cargo")
        });
    let mut candidates = Vec::new();
    for registry in std::fs::read_dir(home.join("registry/src")).unwrap() {
        let registry = registry.unwrap();
        if !registry
            .file_name()
            .to_string_lossy()
            .starts_with("index.crates.io-")
        {
            continue;
        }
        let path = registry
            .path()
            .join(format!("ferroui-reactiveui-{version}"));
        if let Ok(text) = std::fs::read_to_string(path.join("Cargo.toml")) {
            let package = text
                .split("[package]")
                .nth(1)
                .unwrap()
                .split("\n[")
                .next()
                .unwrap();
            if entry(package, "name").as_deref() == Some("ferroui-reactiveui")
                && entry(package, "version").as_deref() == Some(version.as_str())
            {
                candidates.push(path);
            }
        }
    }
    assert_eq!(
        candidates.len(),
        1,
        "Resolve the locked published command source explicitly"
    );
    scan_command(&candidates[0], version)
}

/// Scans the command contract of the ReactiveUI crate in the directory `manifest`.
fn scan_command(manifest: &std::path::Path, version: String) -> Command {
    let root = ferroui_build_scan::export::source_root(manifest, None).unwrap();
    let mut options = ScanOptions::new("ferroui_reactiveui", root);
    options.extern_crates = ["ferroui_system", "ferroui_reactive", "ferroui_splat"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let scan = scan_crate(&options);
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let declarations = native_adapter::read_source_declarations(&scan).unwrap();
    let path = scan
        .normalise(&scan.model.crate_name, "::ferroui_reactiveui::ICommand")
        .unwrap();
    assert!(path.is_resolved());
    let models = ferroui_build_scan::model_set::ModelSet::new(vec![scan.model.clone()]);
    let path = RustType::resolved(&models.canonical(&path.text));
    let source = declarations.definition(&path).unwrap().clone();
    assert_eq!(source.kind, native_adapter::RustDeclarationKind::Trait);
    assert_eq!(
        source.visibility,
        native_adapter::RustDeclarationVisibility::Public
    );
    assert!(source.cfg.is_empty() && source.parameters.is_empty() && source.supertraits.is_empty());
    // ICommand is the actual method/event-only contract. It has no logical
    // property declarations; this is a real source-backed interface header,
    // not an empty substitute for an unported class or runtime object.
    let metadata = TypeModel::new(
        "ICommand",
        TypeKind::Interface,
        RustType::resolved(&source.path.text),
        source.module.as_str(),
    );
    Command {
        scan,
        metadata,
        source,
        version,
    }
}
