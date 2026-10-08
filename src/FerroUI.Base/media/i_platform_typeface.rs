use std::any::Any;
use std::io::Read;

use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IFontMemory};

/// Represents a platform-specific typeface: a font face as loaded by the
/// font backend, exposing its identity and raw table data.
pub trait IPlatformTypeface: IFontMemory {
    /// Gets the family name of the font.
    fn family_name(&self) -> String;

    /// Gets the designed weight of the font.
    fn weight(&self) -> FontWeight;

    /// Gets the style of the font.
    fn style(&self) -> FontStyle;

    /// Gets the designed stretch of the font.
    fn stretch(&self) -> FontStretch;

    /// Gets the algorithmic style simulations applied to this typeface.
    fn font_simulations(&self) -> FontSimulations;

    /// Attempts to obtain a stream over the raw font data.
    fn try_get_stream(&self) -> Option<Box<dyn Read>>;

    /// Lets the backend that created the typeface recover its concrete type
    /// (C#'s type test `PlatformTypeface is SkiaTypeface`).
    fn as_any(&self) -> &dyn Any;
}
