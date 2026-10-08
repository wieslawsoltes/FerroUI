/// Options that influence how a single glyph is drawn, independent of variable-font
/// axis configuration: which CPAL palette to use for color glyphs, and which bitmap
/// strike size to prefer for bitmap-based glyphs.
///
/// All properties are optional. `None` means "use the font's default" (palette 0
/// for color glyphs; no bitmap strike preference). Concerns specific to variable fonts
/// (axis coordinates, named instances) live on `FontVariationSettings`.
///
/// A record with init-only properties: a value is built from [`GlyphDrawingOptions::new`]
/// with [`GlyphDrawingOptions::with_palette_index`] and
/// [`GlyphDrawingOptions::with_pixel_size`], which validate as the property initialisers do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlyphDrawingOptions {
    palette_index: Option<i32>,
    pixel_size: Option<i32>,
}

impl GlyphDrawingOptions {
    /// The instance representing "use the font's defaults".
    pub const DEFAULT: GlyphDrawingOptions = GlyphDrawingOptions { palette_index: None, pixel_size: None };

    /// Creates options that use the font's defaults.
    pub const fn new() -> GlyphDrawingOptions {
        Self::DEFAULT
    }

    /// Gets the optional `CPAL` palette index used to resolve colors for
    /// COLR v0 / COLR v1 glyphs.
    ///
    /// When `None`, the font's default palette (palette 0) is used.
    pub fn palette_index(&self) -> Option<i32> {
        self.palette_index
    }

    /// Returns a copy of the options with the given palette index.
    ///
    /// # Panics
    ///
    /// When the value is negative (`ArgumentOutOfRangeException`).
    pub fn with_palette_index(mut self, value: Option<i32>) -> GlyphDrawingOptions {
        if let Some(v) = value {
            if v < 0 {
                panic!("PaletteIndex must be non-negative. (Parameter 'value')\nActual value was {v}.");
            }
        }

        self.palette_index = value;
        self
    }

    /// Gets the optional pixel size used to select a bitmap strike from `sbix`,
    /// `CBDT` or `EBDT` tables.
    ///
    /// When `None`, no bitmap strike is selected and the font's outline (or
    /// color-layer) representation is used instead.
    pub fn pixel_size(&self) -> Option<i32> {
        self.pixel_size
    }

    /// Returns a copy of the options with the given pixel size.
    ///
    /// # Panics
    ///
    /// When the value is less than 1 (`ArgumentOutOfRangeException`).
    pub fn with_pixel_size(mut self, value: Option<i32>) -> GlyphDrawingOptions {
        if let Some(v) = value {
            if v < 1 {
                panic!("PixelSize must be at least 1. (Parameter 'value')\nActual value was {v}.");
            }
        }

        self.pixel_size = value;
        self
    }
}

// Port of `GlyphDrawingOptionsTests`. The record is a class upstream and a `Copy` value here,
// so the assertions on the identity of instances (`Assert.Same`, `Assert.NotSame`) have no
// counterpart: the tests that have them keep their other assertions.
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::panic::catch_unwind;

    fn hash_code(options: &GlyphDrawingOptions) -> u64 {
        let mut hasher = DefaultHasher::new();
        options.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn default_has_null_palette_index_and_null_pixel_size() {
        let options = GlyphDrawingOptions::DEFAULT;

        assert_eq!(None, options.palette_index());
        assert_eq!(None, options.pixel_size());
    }

    // A value has no identity: the default is the same value every time.
    #[test]
    fn default_is_a_singleton() {
        assert_eq!(GlyphDrawingOptions::DEFAULT, GlyphDrawingOptions::DEFAULT);
    }

    #[test]
    fn parameterless_constructor_produces_an_instance_equal_to_default() {
        // The record's equality contract should treat a freshly-constructed
        // instance with no overrides as equal to the Default singleton. This
        // keeps callers from having to compare against Default by reference.
        let fresh = GlyphDrawingOptions::new();

        assert_eq!(GlyphDrawingOptions::DEFAULT, fresh);
        assert_eq!(hash_code(&GlyphDrawingOptions::DEFAULT), hash_code(&fresh));
    }

    #[test]
    fn palette_index_accepts_null() {
        let options = GlyphDrawingOptions::new().with_palette_index(None);

        assert_eq!(None, options.palette_index());
    }

    #[test]
    fn palette_index_accepts_non_negative_values() {
        for value in [0, 1, 99] {
            let options = GlyphDrawingOptions::new().with_palette_index(Some(value));

            assert_eq!(Some(value), options.palette_index());
        }
    }

    #[test]
    fn palette_index_rejects_negative_values() {
        for value in [-1, i32::MIN] {
            assert!(catch_unwind(|| GlyphDrawingOptions::new().with_palette_index(Some(value))).is_err());
        }
    }

    #[test]
    fn pixel_size_accepts_null() {
        let options = GlyphDrawingOptions::new().with_pixel_size(None);

        assert_eq!(None, options.pixel_size());
    }

    #[test]
    fn pixel_size_accepts_positive_values() {
        for value in [1, 16, 256] {
            let options = GlyphDrawingOptions::new().with_pixel_size(Some(value));

            assert_eq!(Some(value), options.pixel_size());
        }
    }

    #[test]
    fn pixel_size_rejects_values_below_one() {
        // A pixel size of zero is meaningless for a bitmap strike; reject it at
        // construction time rather than letting it propagate to renderer code.
        for value in [0, -1, i32::MIN] {
            assert!(catch_unwind(|| GlyphDrawingOptions::new().with_pixel_size(Some(value))).is_err());
        }
    }

    #[test]
    fn equality_is_structural_across_two_records_with_same_values() {
        let a = GlyphDrawingOptions::new().with_palette_index(Some(1)).with_pixel_size(Some(16));
        let b = GlyphDrawingOptions::new().with_palette_index(Some(1)).with_pixel_size(Some(16));

        assert_eq!(a, b);
        assert_eq!(hash_code(&a), hash_code(&b));
    }

    #[test]
    fn equality_distinguishes_different_palette_index() {
        let a = GlyphDrawingOptions::new().with_palette_index(Some(0));
        let b = GlyphDrawingOptions::new().with_palette_index(Some(1));

        assert_ne!(a, b);
    }

    #[test]
    fn equality_distinguishes_different_pixel_size() {
        let a = GlyphDrawingOptions::new().with_pixel_size(Some(16));
        let b = GlyphDrawingOptions::new().with_pixel_size(Some(32));

        assert_ne!(a, b);
    }

    #[test]
    fn with_expression_produces_a_modified_copy() {
        // `record` participation is part of the public contract; ensure callers
        // can derive a tweaked instance.
        let original = GlyphDrawingOptions::new().with_palette_index(Some(1)).with_pixel_size(Some(16));
        let tweaked = original.with_palette_index(Some(2));

        assert_eq!(Some(1), original.palette_index());
        assert_eq!(Some(2), tweaked.palette_index());
        assert_eq!(Some(16), tweaked.pixel_size());
        assert_ne!(original, tweaked);
    }

    #[test]
    fn with_expression_validates_new_values() {
        let original = GlyphDrawingOptions::new().with_palette_index(Some(1));

        assert!(catch_unwind(|| original.with_palette_index(Some(-5))).is_err());
    }
}
