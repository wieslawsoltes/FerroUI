//! Port of `Transform/XamlDiagnosticsHandler.cs`.

use crate::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity, XamlXWellKnownDiagnosticCodes};
use crate::exceptions::XamlError;

/// The `object` passed to `CodeMappings` upstream: an exception, a well known code or a
/// diagnostic id taken from an attribute.
#[derive(Debug, Clone, Copy)]
pub enum XamlDiagnosticCodeSource<'a> {
    Exception(&'a XamlError),
    WellKnown(XamlXWellKnownDiagnosticCodes),
    Id(&'a str),
}

impl XamlDiagnosticCodeSource<'_> {
    /// `object.ToString()` of the code source.
    pub fn to_code_string(&self) -> String {
        match self {
            XamlDiagnosticCodeSource::Exception(e) => e.to_string(),
            XamlDiagnosticCodeSource::WellKnown(code) => code.to_string(),
            XamlDiagnosticCodeSource::Id(id) => id.to_string(),
        }
    }
}

pub type XamlDiagnosticCodeMappings = Box<dyn Fn(&XamlDiagnosticCodeSource<'_>) -> String>;
pub type XamlDiagnosticExceptionFormatter = Box<dyn Fn(&XamlError) -> String>;
pub type XamlDiagnosticCallback = Box<dyn Fn(&XamlDiagnostic) -> XamlDiagnosticSeverity>;

pub struct XamlDiagnosticsHandler {
    pub code_mappings: XamlDiagnosticCodeMappings,
    pub exception_formatter: XamlDiagnosticExceptionFormatter,
    pub handle_diagnostic: Option<XamlDiagnosticCallback>,
}

impl Default for XamlDiagnosticsHandler {
    fn default() -> Self {
        Self {
            code_mappings: Box::new(|code| code.to_code_string()),
            exception_formatter: Box::new(|ex| ex.message()),
            handle_diagnostic: None,
        }
    }
}

impl XamlDiagnosticsHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lets the host decide the final severity; it never goes below the diagnostic's minimum.
    pub fn report_diagnostic(&self, diagnostic: &XamlDiagnostic) -> XamlDiagnosticSeverity {
        let severity = match &self.handle_diagnostic {
            Some(handler) => handler(diagnostic),
            None => diagnostic.severity,
        };
        if severity > diagnostic.min_severity {
            severity
        } else {
            diagnostic.min_severity
        }
    }
}
