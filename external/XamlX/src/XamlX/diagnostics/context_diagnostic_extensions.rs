//! Port of `Diagnostics/ContextDiagnosticExtensions.cs`.

use std::rc::Rc;

use crate::ast::{IXamlAstNode, IXamlLineInfo};
use crate::exceptions::{XamlError, XamlParseException, XamlResult};
use crate::transform::{AstTransformationContext, XamlDiagnosticCodeSource};

use super::{XamlDiagnostic, XamlDiagnosticSeverity};

impl AstTransformationContext {
    /// `ReportDiagnostic(diagnosticCode, severity, title, offender, minSeverity)`.
    pub fn report_diagnostic_at(
        &self,
        diagnostic_code: &str,
        severity: XamlDiagnosticSeverity,
        title: &str,
        offender: Option<&dyn IXamlLineInfo>,
        min_severity: XamlDiagnosticSeverity,
    ) -> XamlResult<()> {
        let mut diagnostic =
            XamlDiagnostic::with_line_info(diagnostic_code, severity, title, offender);
        diagnostic.document = self.current_document();
        diagnostic.min_severity = min_severity;
        self.report_diagnostic(diagnostic, true)?;
        Ok(())
    }

    /// `ReportTransformError(title, offender)`: reports the error and returns the offender.
    pub fn report_transform_error_node(
        &self,
        title: &str,
        offender: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let line_info = offender.clone();
        self.report_transform_error(title, Some(&*line_info), offender)
    }

    /// `ReportTransformError<TReturn>(title, offender, ret)`.
    pub fn report_transform_error<TReturn>(
        &self,
        title: &str,
        offender: Option<&dyn IXamlLineInfo>,
        ret: TReturn,
    ) -> XamlResult<TReturn> {
        self.report_error(XamlError::transform_exception(title, offender), ret)
    }

    /// `ReportError<TReturn>(exception, ret)`.
    pub fn report_error<TReturn>(&self, exception: XamlError, ret: TReturn) -> XamlResult<TReturn> {
        let diagnostic = exception_to_diagnostic(&exception, self);
        self.report_diagnostic(diagnostic, true)?;
        Ok(ret)
    }
}

/// `Exception.ToDiagnostic(context)`.
pub fn exception_to_diagnostic(
    exception: &XamlError,
    context: &AstTransformationContext,
) -> XamlDiagnostic {
    let handler = &context.configuration().diagnostics_handler;
    let code = (handler.code_mappings)(&XamlDiagnosticCodeSource::Exception(exception));
    let title = (handler.exception_formatter)(exception);
    let mut diagnostic = XamlDiagnostic::new(
        code,
        XamlDiagnosticSeverity::Error,
        title,
        exception.line_number(),
        exception.line_position(),
    );
    diagnostic.document = exception.document().map(str::to_string);
    diagnostic.min_severity = XamlDiagnosticSeverity::Error;
    diagnostic.inner_exception = Some(Box::new(exception.clone()));
    diagnostic
}

impl XamlDiagnostic {
    /// `XamlDiagnostic.ToException()`.
    pub fn to_exception(&self) -> XamlError {
        if let Some(inner) = &self.inner_exception {
            if inner.is_xml_exception() || inner.is_type_system_exception() {
                return (**inner).clone();
            }
        }
        let mut e = XamlParseException::new(
            self.title.clone(),
            self.line(),
            self.position(),
            self.inner_exception.as_deref().cloned(),
        );
        e.document = self.document.clone();
        XamlError::Parse(e)
    }
}

/// `IEnumerable<XamlDiagnostic>.ThrowExceptionIfAnyError()`.
pub fn throw_exception_if_any_error(diagnostics: &[XamlDiagnostic]) -> XamlResult<()> {
    let mut errors: Vec<XamlError> = diagnostics
        .iter()
        .filter(|d| d.severity >= XamlDiagnosticSeverity::Error)
        .map(|d| d.to_exception())
        .collect();
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.remove(0)),
        _ => Err(XamlError::Aggregate(errors)),
    }
}
