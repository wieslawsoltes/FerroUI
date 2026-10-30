use crate::media::{FontFallbackMapping, Typeface};
use crate::utilities::CultureInfo;

/// Optional provider capability for mapping a logical UTF-16 prefix to an
/// actual glyph face and an independent font-size multiplier.
///
/// The family-metrics formatter queries this through
/// [`super::IFontManagerImpl::font_fallback_mapper`]. The current framework
/// resolver remains the fallback with scale one. Configured fallback lists,
/// aliases, and keyed embedded/composite families retain that resolver.
/// Mappings belong to the resulting shaped runs. A provider whose font
/// configuration changes must arrange the usual text-layout/cache invalidation;
/// the formatter does not query it again while reusing a valid shaped cache.
pub trait IFontFallbackMapper: 'static {
    /// Maps the beginning of `text`, in logical order even for a right-to-left
    /// run. The returned length is measured in UTF-16 code units. It must be a
    /// nonempty prefix ending at a scalar and grapheme boundary.
    ///
    /// `typeface` supplies the requested system family, style, weight and
    /// stretch; `culture` supplies the run locale. Return the actual selected
    /// glyph face, not a family name that must be resolved a second time.
    ///
    /// `None` declines this prefix. Invalid ranges, unavailable coverage, or
    /// an unusable scaled em size also decline it. The formatter then advances
    /// by one whole grapheme through its existing resolver, so a
    /// later prefix can still be mapped. Providers should return an explicit
    /// scale-one mapping when the same face can cover a longer unscaled range.
    fn try_map_characters(
        &self,
        text: &[u16],
        typeface: &Typeface,
        culture: Option<&CultureInfo>,
    ) -> Option<FontFallbackMapping>;
}
