use std::any::Any;
use std::io::Read;

use crate::media::{FontSimulations, FontStretch, FontStyle, FontVariationSettings, FontWeight, IFontMemory};

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

    /// Gets the position of the typeface in the variation space of a variable font, as values
    /// of its axes in their own units (a weight of 700 for `wght`). The text shaper shapes the
    /// glyphs at this position. Empty, the default, for a font that is not variable or a
    /// typeface at the default instance of one.
    fn variation_settings(&self) -> FontVariationSettings {
        FontVariationSettings::empty()
    }

    /// The typeface of the same font at another position in the variation space: the axes of
    /// `settings` take their values, the other axes keep the values of this typeface. `None`,
    /// the default, where the backend cannot make one (the font is then used as it is). The
    /// optical size of a text is chosen with it (`GlyphTypeface::at_optical_size`).
    fn with_variation_settings(&self, settings: &FontVariationSettings) -> Option<std::rc::Rc<dyn IPlatformTypeface>> {
        let _ = settings;
        None
    }

    /// Attempts to obtain a stream over the raw font data.
    fn try_get_stream(&self) -> Option<Box<dyn Read>>;

    /// Lets the backend that created the typeface recover its concrete type
    /// (C#'s type test `PlatformTypeface is SkiaTypeface`).
    fn as_any(&self) -> &dyn Any;
}
