use std::any::Any;
use std::rc::Rc;

use crate::media::fonts::font_collection_base::IFontCollectionBase;
use crate::media::{FontFamily, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use crate::utilities::{CultureInfo, Uri};

/// Represents a collection of font families and provides methods for
/// retrieving font-related information, such as glyph typefaces and fallback
/// characters.
///
/// The collection is a read-only list of [`FontFamily`] ([`count`], [`get`],
/// [`font_families`]). Handles are `Rc<dyn IFontCollection>`.
///
/// A custom collection either implements this trait directly, or implements
/// [`FontCollectionBaseImpl`](crate::media::fonts::FontCollectionBaseImpl) to
/// reuse the matching, fallback and caching logic of
/// [`FontCollectionBase`](crate::media::fonts::FontCollectionBase) (this
/// trait is then implemented automatically).
///
/// [`count`]: IFontCollection::count
/// [`get`]: IFontCollection::get
/// [`font_families`]: IFontCollection::font_families
pub trait IFontCollection: 'static {
    /// Gets the unique identifier for the font collection.
    fn key(&self) -> Uri;

    /// The number of font families in the collection.
    fn count(&self) -> usize;

    /// The font family at `index`.
    ///
    /// Panics when `index` is out of range.
    fn get(&self, index: usize) -> FontFamily;

    /// A snapshot of the font families of the collection, in order.
    fn font_families(&self) -> Vec<FontFamily>;

    /// Try to get a glyph typeface for given parameters.
    fn try_get_glyph_typeface(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>>;

    /// Tries to match a specified character to a [`Typeface`] that supports
    /// specified font properties.
    ///
    /// `family_name` is optional and used for fallback lookup.
    fn try_match_character(
        &self,
        codepoint: i32,
        font_style: FontStyle,
        font_weight: FontWeight,
        font_stretch: FontStretch,
        family_name: Option<&str>,
        culture: Option<&CultureInfo>,
    ) -> Option<Typeface>;

    /// Tries to get a list of typefaces for the specified family name.
    fn try_get_family_typefaces(&self, family_name: &str) -> Option<Vec<Typeface>>;

    /// Try to get a synthetic glyph typeface for given parameters.
    fn try_create_synthetic_glyph_typeface(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>>;

    /// Attempts to retrieve the glyph typeface that most closely matches the
    /// specified font family name, style, weight, and stretch.
    ///
    /// This method searches for a glyph typeface in the font collection cache
    /// that matches the specified parameters. If an exact match is not found,
    /// fallback mechanisms are applied to find the closest match based on the
    /// specified style, weight, and stretch. If no suitable match is found,
    /// `None` is returned.
    fn try_get_nearest_match(
        &self,
        family_name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<GlyphTypeface>>;

    /// Releases the glyph typefaces held by the collection (C# `IDisposable.Dispose`).
    fn dispose(&self);

    /// The collection viewed as a `FontCollectionBase` derived collection, if
    /// it is one (C# `collection is FontCollectionBase`).
    fn as_font_collection_base(&self) -> Option<&dyn IFontCollectionBase> {
        None
    }

    /// Lets callers recover the concrete collection type.
    fn as_any(&self) -> &dyn Any;
}

/// Font collections compare by reference, so that a handle can be an untyped
/// value (what a binding to the fonts of the font manager delivers).
impl PartialEq for dyn IFontCollection {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
