//! The diagnostics of a build: what upstream's build task does with the diagnostics of the
//! compiler (`XamlCompilerTaskExecutor`: every diagnostic of the transform goes through
//! the filter, which states its severity, and is logged with its code, its document and
//! its position; `ReportDiagnostics`, `Extensions.LogDiagnostic`).

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_markup_xaml_loader::rust_emitter::DiagnosticHandler;
use xamlx::{XamlDiagnostic, XamlDiagnosticSeverity};

use crate::xaml_compiler_diagnostics_filter::XamlCompilerDiagnosticsFilter;

/// The most diagnostics of one document a build reports (`maxErrorsPerDocument`).
const MAX_DIAGNOSTICS_PER_DOCUMENT: usize = 100;

/// Where a diagnostic is: the document, with the line and the position when both are
/// known (`Views/Main.xaml(12,8)`), as a build tool writes it.
fn place(document: &str, line: Option<i32>, position: Option<i32>) -> String {
    match (line, position) {
        (Some(line), Some(position)) if !document.is_empty() => format!("{document}({line},{position}): "),
        _ if !document.is_empty() => format!("{document}: "),
        _ => String::new(),
    }
}

/// The text of an error with its code: `<document>: error <code>: <message>`.
pub(crate) fn error(code: &str, document: &str, message: &str) -> String {
    format!("{}error {code}: {message}", place(document, None, None))
}

/// The diagnostics of the transforms of a build, with the filter that states their
/// severities.
pub(crate) struct Diagnostics {
    filter: Rc<XamlCompilerDiagnosticsFilter>,
    warnings_as_errors: bool,
    collected: Rc<RefCell<Vec<XamlDiagnostic>>>,
}

impl Diagnostics {
    pub(crate) fn new(filter: XamlCompilerDiagnosticsFilter, warnings_as_errors: bool) -> Self {
        Self { filter: Rc::new(filter), warnings_as_errors, collected: Rc::new(RefCell::new(Vec::new())) }
    }

    /// The handler of the diagnostics of a transform (`HandleDiagnostic` of upstream's
    /// task): the filter states the severity, and the diagnostic is kept with it for
    /// [`report`](Self::report).
    pub(crate) fn handler(&self) -> DiagnosticHandler {
        let (filter, collected) = (self.filter.clone(), self.collected.clone());
        Rc::new(move |diagnostic: &XamlDiagnostic| {
            let new_severity = filter.handle(diagnostic);
            let mut diagnostic = diagnostic.clone();
            diagnostic.severity = new_severity;
            collected.borrow_mut().push(diagnostic);
            new_severity
        })
    }

    /// Reports the diagnostics the handler kept since the last call, at most
    /// [`MAX_DIAGNOSTICS_PER_DOCUMENT`] of a document: a warning as a `cargo::warning=`
    /// line of `lines`, an error as an error of the build, a silenced one not at all.
    /// Returns whether one of them is an error of the transform (`hasAnyError` of
    /// upstream: a warning made an error by [`Build::warnings_as_errors`](crate::Build::warnings_as_errors)
    /// fails the build and not the transform).
    pub(crate) fn report(&self, lines: &mut Vec<String>, errors: &mut Vec<String>) -> bool {
        let diagnostics = std::mem::take(&mut *self.collected.borrow_mut());
        let has_any_error = diagnostics.iter().any(|diagnostic| diagnostic.severity >= XamlDiagnosticSeverity::Error);
        let mut reported: Vec<(Option<String>, usize)> = Vec::new();
        for diagnostic in &diagnostics {
            let count = match reported.iter_mut().find(|(document, _)| *document == diagnostic.document) {
                Some((_, count)) => count,
                None => {
                    reported.push((diagnostic.document.clone(), 0));
                    &mut reported.last_mut().expect("the entry that was pushed").1
                }
            };
            *count += 1;
            if *count > MAX_DIAGNOSTICS_PER_DOCUMENT {
                continue;
            }
            let place = place(diagnostic.document.as_deref().unwrap_or_default(), diagnostic.line_number, diagnostic.line_position);
            self.log(diagnostic.severity, &place, &diagnostic.code, &diagnostic.title, lines, errors);
        }
        has_any_error
    }

    /// Reports a warning of the build that is not a diagnostic of a transform, with the
    /// severity the filter states for its code.
    pub(crate) fn warning(&self, code: &str, document: &str, message: &str, lines: &mut Vec<String>, errors: &mut Vec<String>) {
        let severity = self.filter.handle_code(XamlDiagnosticSeverity::Warning, code);
        self.log(severity, &place(document, None, None), code, message, lines, errors);
    }

    /// `LogDiagnostic`: nothing for a silenced diagnostic, a warning, or an error.
    fn log(&self, severity: XamlDiagnosticSeverity, place: &str, code: &str, message: &str, lines: &mut Vec<String>, errors: &mut Vec<String>) {
        let message = message.replace(['\r', '\n'], " ");
        match severity {
            XamlDiagnosticSeverity::None => {}
            XamlDiagnosticSeverity::Warning if !self.warnings_as_errors => lines.push(format!("cargo::warning={place}warning {code}: {message}")),
            _ => errors.push(format!("{place}error {code}: {message}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn diagnostic(code: &str, severity: XamlDiagnosticSeverity, title: &str, document: Option<&str>, at: Option<(i32, i32)>) -> XamlDiagnostic {
        let mut diagnostic = XamlDiagnostic::new(code, severity, title, at.map(|at| at.0), at.map(|at| at.1));
        diagnostic.document = document.map(str::to_string);
        diagnostic
    }

    /// What the handler returns for each diagnostic, and what the build then reports.
    fn run(filter: XamlCompilerDiagnosticsFilter, warnings_as_errors: bool, given: &[XamlDiagnostic]) -> (Vec<XamlDiagnosticSeverity>, bool, Vec<String>, Vec<String>) {
        let diagnostics = Diagnostics::new(filter, warnings_as_errors);
        let handler = diagnostics.handler();
        let severities = given.iter().map(|diagnostic| handler(diagnostic)).collect();
        let (mut lines, mut errors) = (Vec::new(), Vec::new());
        let has_any_error = diagnostics.report(&mut lines, &mut errors);
        // Reported once.
        assert!(!diagnostics.report(&mut lines, &mut errors));
        (severities, has_any_error, lines, errors)
    }

    fn given() -> Vec<XamlDiagnostic> {
        vec![
            diagnostic("FRN2208", XamlDiagnosticSeverity::Warning, "An item container\nin a template", Some("Views/Main.xaml"), Some((12, 8))),
            diagnostic("FRN5001", XamlDiagnosticSeverity::Warning, "Obsolete", Some("Views/Main.xaml"), None),
            diagnostic("FRN2000", XamlDiagnosticSeverity::Error, "Unable to resolve type Missing", Some("Other.xaml"), Some((3, 4))),
            diagnostic("FRN2203", XamlDiagnosticSeverity::Warning, "Duplicate setter", None, Some((1, 2))),
        ]
    }

    /// Not from upstream: without EditorConfig files a diagnostic keeps its severity and
    /// is reported with its code, its document and its position, as upstream logs it.
    #[test]
    fn diagnostics_are_reported_with_code_document_and_position() {
        let (severities, has_any_error, lines, errors) = run(XamlCompilerDiagnosticsFilter::new(None), false, &given());
        use XamlDiagnosticSeverity::{Error, Warning};
        assert_eq!(severities, [Warning, Warning, Error, Warning]);
        assert!(has_any_error);
        assert_eq!(
            lines,
            [
                "cargo::warning=Views/Main.xaml(12,8): warning FRN2208: An item container in a template",
                "cargo::warning=Views/Main.xaml: warning FRN5001: Obsolete",
                "cargo::warning=warning FRN2203: Duplicate setter",
            ]
        );
        assert_eq!(errors, ["Other.xaml(3,4): error FRN2000: Unable to resolve type Missing"]);
    }

    /// Not from upstream: the severities of the EditorConfig files are the ones the
    /// transform continues with and the build reports; warnings as errors applies to the
    /// warnings that are left and does not fail the transform.
    #[test]
    fn editor_config_and_warnings_as_errors_decide_the_severity() {
        let file = std::env::temp_dir().join(format!("ferroui-build-diagnostics-{}.editorconfig", std::process::id()));
        fs::write(
            &file,
            "[*.xaml]\nferro_xaml_diagnostic.FRN2208.severity = error\nferro_xaml_diagnostic.FRN5001.severity = none\nferro_xaml_diagnostic.FRN2000.severity = warning\n",
        )
        .expect("the EditorConfig file of the test");
        let filter = || XamlCompilerDiagnosticsFilter::new(Some(vec![file.clone()]));
        use XamlDiagnosticSeverity::{Error, None as Silent, Warning};

        let (severities, has_any_error, lines, errors) = run(filter(), false, &given());
        assert_eq!(severities, [Error, Silent, Warning, Warning]);
        assert!(has_any_error);
        assert_eq!(
            lines,
            ["cargo::warning=Other.xaml(3,4): warning FRN2000: Unable to resolve type Missing", "cargo::warning=warning FRN2203: Duplicate setter"]
        );
        assert_eq!(errors, ["Views/Main.xaml(12,8): error FRN2208: An item container in a template"]);

        let (severities, has_any_error, lines, errors) = run(filter(), true, &given()[1..]);
        assert_eq!(severities, [Silent, Warning, Warning]);
        assert!(!has_any_error, "a warning reported as an error is not an error of the transform");
        assert_eq!(lines, Vec::<String>::new());
        assert_eq!(errors, ["Other.xaml(3,4): error FRN2000: Unable to resolve type Missing", "error FRN2203: Duplicate setter"]);

        // A warning of the build itself asks the filter by its code.
        let diagnostics = Diagnostics::new(filter(), false);
        let (mut lines, mut errors) = (Vec::new(), Vec::new());
        diagnostics.warning("FRN3001", "Views/Main.xaml", "not reachable", &mut lines, &mut errors);
        diagnostics.warning("FRN5001", "Views/Main.xaml", "silenced", &mut lines, &mut errors);
        diagnostics.warning("FRN2208", "Views/Main.xaml", "made an error", &mut lines, &mut errors);
        assert_eq!(lines, ["cargo::warning=Views/Main.xaml: warning FRN3001: not reachable"]);
        assert_eq!(errors, ["Views/Main.xaml: error FRN2208: made an error"]);
        let _ = fs::remove_file(&file);
    }

    /// Not from upstream's tests: at most a hundred diagnostics of a document are reported
    /// (`maxErrorsPerDocument`).
    #[test]
    fn diagnostics_of_a_document_are_capped() {
        let many: Vec<XamlDiagnostic> = (0..MAX_DIAGNOSTICS_PER_DOCUMENT as i32 + 5)
            .map(|line| diagnostic("FRN2203", XamlDiagnosticSeverity::Warning, "Duplicate setter", Some("A.xaml"), Some((line, 1))))
            .chain([diagnostic("FRN2203", XamlDiagnosticSeverity::Warning, "Duplicate setter", Some("B.xaml"), Some((1, 1)))])
            .collect();
        let (_, _, lines, errors) = run(XamlCompilerDiagnosticsFilter::new(None), false, &many);
        assert_eq!(lines.len(), MAX_DIAGNOSTICS_PER_DOCUMENT + 1);
        assert_eq!(lines.last().map(String::as_str), Some("cargo::warning=B.xaml(1,1): warning FRN2203: Duplicate setter"));
        assert!(errors.is_empty());
    }
}
