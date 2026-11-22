//! The export trie of the executables of an application on macOS and iOS (#1049).
//!
//! A Mach-O executable lists its global symbols in an export trie that nothing looks up unless
//! a library loaded into the process resolves names from the executable. Outside the dev profile
//! an executable is linked without it (`-Wl,-no_exported_symbols`). The argument only reaches the
//! binary targets of the package, so a `cdylib` or `staticlib` keeps its exports.
//!
//! An application with plug-ins that resolve names from the executable keeps the trie with
//! `[package.metadata.ferroui] executable-exports = true`.

use std::env;
use std::fs;
use std::path::Path;

use crate::theme_roots::MANIFEST_TABLE;

/// The switch of the manifest: `executable-exports = true` keeps the export trie.
pub const EXECUTABLE_EXPORTS_KEY: &str = "executable-exports";

const LINE: &str = "cargo::rustc-link-arg-bins=-Wl,-no_exported_symbols";

/// Whether a build for this target and Cargo profile leaves the trie out.
fn applies(vendor: Option<&str>, os: Option<&str>, profile: Option<&str>) -> bool {
    vendor == Some("apple") && matches!(os, Some("macos" | "ios")) && matches!(profile, Some(p) if p != "debug")
}

/// Whether the manifest states that the executables keep their exports.
fn keeps_exports(manifest: &toml::Table) -> Result<bool, String> {
    let value = manifest
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("ferroui"))
        .and_then(|table| table.get(EXECUTABLE_EXPORTS_KEY));
    match value {
        None => Ok(false),
        Some(toml::Value::Boolean(value)) => Ok(*value),
        Some(_) => Err(format!("[{MANIFEST_TABLE}] {EXECUTABLE_EXPORTS_KEY} must be true or false")),
    }
}

/// Whether the package has a binary target. Cargo refuses `rustc-link-arg-bins` for one without.
fn has_binary(manifest: &toml::Table, directory: &Path) -> bool {
    if manifest.get("bin").and_then(toml::Value::as_array).is_some_and(|targets| !targets.is_empty()) {
        return true;
    }
    let automatic = manifest
        .get("package")
        .and_then(|package| package.get("autobins"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    automatic
        && (directory.join("src/main.rs").is_file()
            || fs::read_dir(directory.join("src/bin")).is_ok_and(|mut entries| entries.next().is_some()))
}

/// The lines for Cargo, from the manifest `text` of the package in `directory`.
fn lines_for(text: &str, directory: &Path) -> Result<Vec<String>, String> {
    let manifest = text.parse::<toml::Table>().map_err(|error| format!("Cargo.toml: {error}"))?;
    if keeps_exports(&manifest)? || !has_binary(&manifest, directory) {
        return Ok(Vec::new());
    }
    Ok(vec![LINE.to_string()])
}

/// The lines for the build script that is running; none outside a build for macOS or iOS.
pub(crate) fn lines(directory: &Path) -> Result<Vec<String>, String> {
    let variable = |name: &str| env::var(name).ok();
    if !applies(
        variable("CARGO_CFG_TARGET_VENDOR").as_deref(),
        variable("CARGO_CFG_TARGET_OS").as_deref(),
        variable("PROFILE").as_deref(),
    ) {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(directory.join("Cargo.toml")).map_err(|error| format!("Cargo.toml: {error}"))?;
    lines_for(&text, directory)
}

/// Links the executables of the package without an export trie, for a build script that does not
/// run [`Build`](crate::Build), which does this itself.
///
/// # Panics
/// Panics outside a build script (`CARGO_MANIFEST_DIR` is missing).
pub fn strip_executable_exports() {
    let directory = env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set: not a build script");
    let directory = Path::new(&directory);
    println!("cargo::rerun-if-changed={}", directory.join("Cargo.toml").display());
    match lines(directory) {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(error) => {
            eprintln!("error: {error}");
            println!("cargo::error={error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BINARY: &str = "[package]\nname = \"app\"\n\n[[bin]]\nname = \"app\"\npath = \"main.rs\"\n";

    #[test]
    fn only_macos_and_ios_outside_the_dev_profile() {
        assert!(applies(Some("apple"), Some("macos"), Some("release")));
        assert!(applies(Some("apple"), Some("ios"), Some("release")));
        assert!(!applies(Some("apple"), Some("macos"), Some("debug")));
        assert!(!applies(Some("apple"), Some("tvos"), Some("release")));
        assert!(!applies(Some("unknown"), Some("linux"), Some("release")));
        assert!(!applies(None, None, None));
    }

    #[test]
    fn a_binary_target_gets_the_argument() {
        let lines = lines_for(BINARY, Path::new("/nonexistent")).expect("a manifest");
        assert_eq!(lines, vec![LINE.to_string()]);
    }

    #[test]
    fn a_library_gets_no_argument() {
        let text = "[package]\nname = \"library\"\n\n[lib]\ncrate-type = [\"cdylib\", \"staticlib\"]\n";
        assert!(lines_for(text, Path::new("/nonexistent")).expect("a manifest").is_empty());
    }

    #[test]
    fn the_automatic_binary_is_found_unless_turned_off() {
        let directory = env::temp_dir().join(format!("ferroui-build-exports-{}", std::process::id()));
        fs::create_dir_all(directory.join("src")).expect("a directory");
        fs::write(directory.join("src/main.rs"), "fn main() {}\n").expect("a file");
        let automatic = "[package]\nname = \"app\"\n";
        assert_eq!(lines_for(automatic, &directory).expect("a manifest").len(), 1);
        let off = "[package]\nname = \"app\"\nautobins = false\n";
        assert!(lines_for(off, &directory).expect("a manifest").is_empty());
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_manifest_keeps_the_exports() {
        let text = format!("{BINARY}\n[package.metadata.ferroui]\nexecutable-exports = true\n");
        assert!(lines_for(&text, Path::new("/nonexistent")).expect("a manifest").is_empty());
        let text = format!("{BINARY}\n[package.metadata.ferroui]\nexecutable-exports = false\n");
        assert_eq!(lines_for(&text, Path::new("/nonexistent")).expect("a manifest").len(), 1);
    }

    #[test]
    fn a_wrong_value_of_the_key_is_an_error() {
        let text = format!("{BINARY}\n[package.metadata.ferroui]\nexecutable-exports = \"yes\"\n");
        assert_eq!(
            lines_for(&text, Path::new("/nonexistent")),
            Err("[package.metadata.ferroui] executable-exports must be true or false".to_string())
        );
    }
}
