use std::io::Read;
use std::rc::Rc;

use crate::media::{FontSimulations, FontStretch, FontStyle, FontWeight, IPlatformTypeface, Typeface};
use crate::utilities::Locale;
use super::IFontFallbackMapper;

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
        culture: Option<&Locale>,
    ) -> Option<Rc<dyn IPlatformTypeface>>;

    /// Optional range mapping with an independent fallback font scale.
    /// Existing providers return `None` and retain the character matcher with
    /// scale one. The rich family-metrics policy queries this capability;
    /// the default formatter does not.
    fn font_fallback_mapper(&self) -> Option<&dyn IFontFallbackMapper> {
        None
    }

    /// Whether `family_name` is a name the backend resolves to faces of `face_family_name` without
    /// either being installed under the other: an alias of the system, such as the private names
    /// of the system UI font. A composite family takes such a face for the alias; a face the
    /// backend substituted for a name it does not know is not one, so the composite family goes
    /// on to its next family. No alias by default.
    fn is_family_alias(&self, _family_name: &str, _face_family_name: &str) -> bool {
        false
    }

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
