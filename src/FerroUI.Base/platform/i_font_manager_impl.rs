use std::io::Read;
use std::rc::Rc;

use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use crate::utilities::CultureInfo;

/// The font backend contract: enumerates installed fonts, creates platform
/// typefaces and matches fallback fonts for characters.
pub trait IFontManagerImpl: 'static {
    /// Gets the system's default font family's name.
    fn get_default_font_family_name(&self) -> String;

    /// Get all installed fonts in the system.
    ///
    /// `check_for_updates`: if `true` the font collection is updated.
    fn get_installed_font_family_names(&self, check_for_updates: bool) -> Vec<String>;

    /// Tries to match a specified character to a typeface that supports
    /// specified font properties.
    ///
    /// Returns the matching platform typeface, or `None` when no font covers
    /// the codepoint.
    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Rc<dyn IPlatformTypeface>>;

    /// Tries to get a glyph typeface for specified parameters.
    fn try_create_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>>;

    /// Tries to create a glyph typeface from specified stream.
    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        font_simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>>;

    /// Tries to get a list of typefaces for the specified family name.
    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>>;

    /// Releases backend resources. Called when the font manager is disposed
    /// (upstream: `(PlatformImpl as IDisposable)?.Dispose()`).
    fn dispose(&self) {}
}
