//! Native FerroUI font fallback, Unicode bidi and text shaping for Pretext.
//!
//! Call [`create_layout`] after configuring the application's FerroUI font and
//! shaping backends. The adapter uses the same shapeable-character selection,
//! bidi algorithm and platform shaper as the framework's text formatter.
//!
//! For direct drawing, [`FerroTextService::shape_native`] retains the actual font
//! faces and returns [`FerroShapedText`] with native glyph runs. Glyph positions
//! use baseline-relative device-independent units; clusters use UTF-16 units.

#![forbid(unsafe_code)]

mod ferro_shaped_text;
mod ferro_text_measurer_factory;

pub use ferro_shaped_text::FerroShapedText;
pub use ferro_text_measurer_factory::{create_layout, FerroTextMeasurerFactory, FerroTextService};

#[cfg(test)]
mod tests;
