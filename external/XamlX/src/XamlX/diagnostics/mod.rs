//! Port of the `Diagnostics/` directory.
//!
//! `TrimmingMessages.cs` only carries .NET trimmer annotations and has no Rust counterpart.

mod context_diagnostic_extensions;
mod xaml_diagnostic;
mod xaml_diagnostic_severity;
mod xaml_x_well_known_diagnostic_codes;

pub use context_diagnostic_extensions::*;
pub use xaml_diagnostic::*;
pub use xaml_diagnostic_severity::*;
pub use xaml_x_well_known_diagnostic_codes::*;
