//! Backend contracts for reusable text preparation and glyph-run output.
//!
//! Layout cursors count graphemes; glyph clusters count UTF-16 code units.
//! Implementations use ordinary Rust ownership instead of explicit disposal.

#![forbid(unsafe_code)]

mod pretext_font_descriptor;
mod text_measurement_contracts;
mod text_shaping_contracts;

pub use pretext_font_descriptor::*;
pub use text_measurement_contracts::*;
pub use text_shaping_contracts::*;

/// A recoverable backend or input error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PretextError {
    /// No glyph shaping backend was configured.
    NoShapingBackend,
    /// A backend could not create or execute a text service.
    Backend(String),
    /// Input is outside the supported contract.
    InvalidInput(String),
}

impl std::fmt::Display for PretextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoShapingBackend => f.write_str("no text shaping backend is configured"),
            Self::Backend(message) | Self::InvalidInput(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PretextError {}
