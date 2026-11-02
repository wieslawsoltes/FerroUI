//! The published crates and this conformance suite exercise the same QML files.
use std::path::{Path, PathBuf};

pub(crate) fn controls(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/Markup/FerroUI.Qml.Controls/tests/fixtures")
        .join(file)
}

pub(crate) fn runtime(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/Markup/FerroUI.Qml/tests/fixtures")
        .join(file)
}
