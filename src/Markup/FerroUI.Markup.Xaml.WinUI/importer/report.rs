//! Migration evidence refers to the original UTF-8 input, not the rewritten document.

use std::fmt;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlSourceSpan {
    pub bytes: Range<usize>,
    /// One-based line and Unicode scalar column in the input.
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinUIXamlDiagnosticSeverity {
    Adaptation,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlDiagnostic {
    /// Stable identifier suitable for automated parity ledgers.
    pub code: &'static str,
    pub severity: WinUIXamlDiagnosticSeverity,
    pub message: String,
    pub span: WinUIXamlSourceSpan,
}

/// A nonoverlapping replacement of an original source range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlEdit {
    pub span: WinUIXamlSourceSpan,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlMigration {
    /// Reviewable candidate. Call `into_xaml` to reject unresolved diagnostics before use.
    pub xaml: String,
    pub edits: Vec<WinUIXamlEdit>,
    pub diagnostics: Vec<WinUIXamlDiagnostic>,
}

impl WinUIXamlMigration {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == WinUIXamlDiagnosticSeverity::Error)
    }

    /// Rejects unresolved semantics. Both the optional runtime entry point and build tools
    /// should use this rather than loading the reviewable candidate directly.
    pub fn into_xaml(self) -> Result<String, WinUIXamlMigrationError> {
        if self.has_errors() {
            Err(WinUIXamlMigrationError {
                diagnostics: self.diagnostics,
            })
        } else {
            Ok(self.xaml)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinUIXamlMigrationError {
    pub diagnostics: Vec<WinUIXamlDiagnostic>,
}

impl fmt::Display for WinUIXamlMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, d) in self
            .diagnostics
            .iter()
            .filter(|d| d.severity == WinUIXamlDiagnosticSeverity::Error)
            .enumerate()
        {
            if index != 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "{} at {}:{}: {}",
                d.code, d.span.line, d.span.column, d.message
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for WinUIXamlMigrationError {}
