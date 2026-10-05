//! Port of `Diagnostics/XamlDiagnosticSeverity.cs`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum XamlDiagnosticSeverity {
    #[default]
    None = 0,
    Warning = 1,
    Error = 2,
    Fatal = 3,
}
