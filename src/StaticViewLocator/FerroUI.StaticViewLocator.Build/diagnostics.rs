//! The diagnostics of the generator (`DiagnosticDescriptor`s of
//! `StaticViewLocatorGenerator.Adapters.cs`).
//!
//! The codes, the titles and the messages are the reference's; a message names Rust items where
//! the reference names C# members. The build fails on an error, as a compilation fails on an error
//! of a source generator. SVL0009 and SVL0010 are additions of the port.

use std::fmt;
use std::path::{Path, PathBuf};

/// How bad a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The build fails.
    Error,
    /// The build goes on; Cargo prints the warning.
    Warning,
}

/// A diagnostic of the generator, with the place in the Rust source when there is one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// `SVL0001` to `SVL0010`.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    /// The file, when the diagnostic is about an item of the sources.
    pub file: Option<PathBuf>,
    /// The line, from 1; 0 when the diagnostic is about the file.
    pub line: usize,
}

impl Diagnostic {
    pub(crate) fn error(code: &'static str, message: impl Into<String>, at: &Location) -> Self {
        Self { code, severity: Severity::Error, message: message.into(), file: at.file.clone(), line: at.line }
    }

    /// The diagnostic as a line of Cargo (`cargo::error=..` or `cargo::warning=..`).
    pub fn cargo_line(&self) -> String {
        match self.severity {
            Severity::Error => format!("cargo::error={self}"),
            Severity::Warning => format!("cargo::warning={self}"),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match &self.file {
            Some(file) if self.line > 0 => write!(f, "{}({}): {severity} {}: {}", file.display(), self.line, self.code, self.message),
            Some(file) => write!(f, "{}: {severity} {}: {}", file.display(), self.code, self.message),
            None => write!(f, "{severity} {}: {}", self.code, self.message),
        }
    }
}

/// Where an item is (`Location` of a symbol).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Location {
    pub file: Option<PathBuf>,
    pub line: usize,
}

impl Location {
    pub(crate) fn new(file: &Path, line: usize) -> Self {
        Self { file: Some(file.to_path_buf()), line }
    }
}

/// SVL0001: a requested adapter needs a crate or a feature that is not there
/// ("'{0}' requires a reference containing '{1}'").
pub const MISSING_ADAPTER_TYPE: &str = "SVL0001";
/// SVL0002: a member of the locator has the name of a generated member and cannot stand for it.
pub const INCOMPATIBLE_ADAPTER_MEMBER: &str = "SVL0002";
/// SVL0003: more than one inferred view maps to the same view model.
pub const AMBIGUOUS_VIEW_MAPPING: &str = "SVL0003";
/// SVL0004: a mapped view is not accessible, is generic or abstract, or has no constructor
/// without parameters.
pub const VIEW_CANNOT_BE_CONSTRUCTED: &str = "SVL0004";
/// SVL0005: the locator cannot be extended by generated source (it is not a struct).
pub const UNSUPPORTED_LOCATOR_TYPE: &str = "SVL0005";
/// SVL0006: a configured mapping contract is not a trait with exactly one view model type.
pub const INVALID_MAPPING_CONTRACT: &str = "SVL0006";
/// SVL0007: mapped traits of a view model resolve to different views.
pub const AMBIGUOUS_FALLBACK_MAPPING: &str = "SVL0007";
/// SVL0008: an open generic view model has no compiled fallback in the generated adapters.
pub const UNSUPPORTED_OPEN_GENERIC_ADAPTER_MAPPING: &str = "SVL0008";
/// SVL0009 (an addition): two locators of a crate generate the same file.
pub const DUPLICATE_LOCATOR_FILE: &str = "SVL0009";
/// SVL0010 (an addition): an option of the attribute or of the build script is not understood.
pub const INVALID_OPTION: &str = "SVL0010";
