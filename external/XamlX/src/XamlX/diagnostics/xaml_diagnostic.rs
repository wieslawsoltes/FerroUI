//! Port of `Diagnostics/XamlDiagnostic.cs`.

use crate::ast::IXamlLineInfo;
use crate::exceptions::XamlError;

use super::XamlDiagnosticSeverity;

#[derive(Debug, Clone)]
pub struct XamlDiagnostic {
    pub code: String,
    pub severity: XamlDiagnosticSeverity,
    pub title: String,
    pub line_number: Option<i32>,
    pub line_position: Option<i32>,
    pub min_severity: XamlDiagnosticSeverity,
    pub document: Option<String>,
    pub inner_exception: Option<Box<XamlError>>,
}

impl XamlDiagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: XamlDiagnosticSeverity,
        title: impl Into<String>,
        line_number: Option<i32>,
        line_position: Option<i32>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            title: title.into(),
            line_number,
            line_position,
            min_severity: XamlDiagnosticSeverity::None,
            document: None,
            inner_exception: None,
        }
    }

    pub fn with_line_info(
        code: impl Into<String>,
        severity: XamlDiagnosticSeverity,
        title: impl Into<String>,
        line_info: Option<&dyn IXamlLineInfo>,
    ) -> Self {
        Self::new(
            code,
            severity,
            title,
            line_info.map(|l| l.line()),
            line_info.map(|l| l.position()),
        )
    }
}

impl IXamlLineInfo for XamlDiagnostic {
    fn line(&self) -> i32 {
        self.line_number.unwrap_or(0)
    }
    fn position(&self) -> i32 {
        self.line_position.unwrap_or(0)
    }
    fn set_line(&self, _value: i32) {}
    fn set_position(&self, _value: i32) {}
}
