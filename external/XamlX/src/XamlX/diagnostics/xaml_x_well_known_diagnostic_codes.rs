//! Port of `Diagnostics/XamlXWellKnownDiagnosticCodes.cs`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XamlXWellKnownDiagnosticCodes {
    Obsolete = 1,
}

impl fmt::Display for XamlXWellKnownDiagnosticCodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XamlXWellKnownDiagnosticCodes::Obsolete => write!(f, "Obsolete"),
        }
    }
}
