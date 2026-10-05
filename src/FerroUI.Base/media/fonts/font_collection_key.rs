use std::cmp::Ordering;

use crate::media::{FontStretch, FontStyle, FontWeight, GlyphTypeface, IPlatformTypeface, Typeface};

/// Represents a unique key for identifying a font within a font collection
/// based on style, weight and stretch.
///
/// Keys order by style, then weight, then stretch (numeric values).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontCollectionKey {
    pub style: FontStyle,
    pub weight: FontWeight,
    pub stretch: FontStretch,
}

impl FontCollectionKey {
    pub const fn new(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        Self { style, weight, stretch }
    }
}

impl Ord for FontCollectionKey {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.style as i32)
            .cmp(&(other.style as i32))
            .then_with(|| self.weight.value().cmp(&other.weight.value()))
            .then_with(|| (self.stretch as i32).cmp(&(other.stretch as i32)))
    }
}

impl PartialOrd for FontCollectionKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// The `ToFontCollectionKey` extension methods (upstream `FontCollectionKeyExtensions.cs`).

impl From<&Typeface> for FontCollectionKey {
    fn from(typeface: &Typeface) -> Self {
        FontCollectionKey::new(typeface.style(), typeface.weight(), typeface.stretch())
    }
}

impl From<&GlyphTypeface> for FontCollectionKey {
    fn from(glyph_typeface: &GlyphTypeface) -> Self {
        FontCollectionKey::new(glyph_typeface.style(), glyph_typeface.weight(), glyph_typeface.stretch())
    }
}

impl From<&dyn IPlatformTypeface> for FontCollectionKey {
    fn from(platform_typeface: &dyn IPlatformTypeface) -> Self {
        FontCollectionKey::new(platform_typeface.style(), platform_typeface.weight(), platform_typeface.stretch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_keys_compare_equal() {
        let a = FontCollectionKey::new(FontStyle::Italic, FontWeight::Bold, FontStretch::Condensed);
        let b = FontCollectionKey::new(FontStyle::Italic, FontWeight::Bold, FontStretch::Condensed);

        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_eq!(a.partial_cmp(&b), Some(Ordering::Equal));
        assert!(a <= b);
        assert!(a >= b);
    }

    #[test]
    fn style_is_the_primary_sort_key() {
        // Normal style, max weight/stretch
        let normal = FontCollectionKey::new(FontStyle::Normal, FontWeight::Black, FontStretch::UltraExpanded);
        // Italic style, min weight/stretch
        let italic = FontCollectionKey::new(FontStyle::Italic, FontWeight::Thin, FontStretch::UltraCondensed);

        assert_eq!(normal.cmp(&italic), Ordering::Less);
        assert_eq!(italic.cmp(&normal), Ordering::Greater);
        assert!(normal < italic);
    }

    #[test]
    fn weight_is_the_secondary_sort_key_when_style_matches() {
        let light = FontCollectionKey::new(FontStyle::Normal, FontWeight::Light, FontStretch::UltraExpanded);
        let bold = FontCollectionKey::new(FontStyle::Normal, FontWeight::Bold, FontStretch::UltraCondensed);

        assert_eq!(light.cmp(&bold), Ordering::Less);
        assert!(light < bold);
    }

    #[test]
    fn stretch_is_the_tertiary_sort_key_when_style_and_weight_match() {
        let condensed = FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Condensed);
        let expanded = FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Expanded);

        assert_eq!(condensed.cmp(&expanded), Ordering::Less);
        assert!(condensed < expanded);
    }

    #[test]
    fn array_sort_produces_lexicographic_order() {
        let mut keys = [
            FontCollectionKey::new(FontStyle::Italic, FontWeight::Normal, FontStretch::Normal),
            FontCollectionKey::new(FontStyle::Normal, FontWeight::Bold, FontStretch::Normal),
            FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Expanded),
            FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Normal),
        ];

        keys.sort();

        assert_eq!(
            keys,
            [
                FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Normal),
                FontCollectionKey::new(FontStyle::Normal, FontWeight::Normal, FontStretch::Expanded),
                FontCollectionKey::new(FontStyle::Normal, FontWeight::Bold, FontStretch::Normal),
                FontCollectionKey::new(FontStyle::Italic, FontWeight::Normal, FontStretch::Normal),
            ]
        );
    }

    #[test]
    fn compare_to_is_consistent_with_equality() {
        let a = FontCollectionKey::new(FontStyle::Oblique, FontWeight::Medium, FontStretch::SemiExpanded);
        let b = FontCollectionKey::new(FontStyle::Oblique, FontWeight::Medium, FontStretch::SemiExpanded);
        let c = FontCollectionKey::new(FontStyle::Oblique, FontWeight::Medium, FontStretch::SemiCondensed);

        assert_eq!(a, b);
        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_ne!(a, c);
        assert_ne!(a.cmp(&c), Ordering::Equal);
    }

    #[test]
    fn keys_are_made_from_typefaces() {
        let typeface = Typeface::from_name_with_style("A", FontStyle::Italic, FontWeight::Bold, FontStretch::Expanded);

        assert_eq!(
            FontCollectionKey::from(&typeface),
            FontCollectionKey::new(FontStyle::Italic, FontWeight::Bold, FontStretch::Expanded)
        );
    }
}
