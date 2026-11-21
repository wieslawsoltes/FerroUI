use crate::glyph_bounds_cache::GlyphBoundsCache;
use crate::skia_sharp_extensions::slant_to_font_style;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontVariation, FontVariationSettings, FontWeight, IFontMemory,
    IPlatformTypeface,
};
use ferroui_base::utilities::ReadOnlyMemory;
use skia_safe::{Font, Typeface};
use std::any::Any;
use std::io::{Cursor, Read};
use std::rc::Rc;

/// A platform typeface over a Skia typeface.
pub struct SkiaTypeface {
    sk_typeface: Typeface,
    font_simulations: FontSimulations,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    /// The glyph bounds its glyph runs measured, by size (`glyph_bounds_cache`).
    glyph_bounds: GlyphBoundsCache,
}

impl SkiaTypeface {
    /// Wraps a Skia typeface, applying the given style simulations when
    /// fonts are created from it.
    pub fn new(typeface: Typeface, font_simulations: FontSimulations) -> Rc<Self> {
        let font_style = typeface.font_style();

        Rc::new(Self {
            weight: FontWeight(*font_style.weight()),
            style: slant_to_font_style(font_style.slant()),
            stretch: FontStretch::from_i32(*font_style.width()).unwrap_or(FontStretch::Normal),
            sk_typeface: typeface,
            font_simulations,
            glyph_bounds: GlyphBoundsCache::default(),
        })
    }

    /// The Skia typeface.
    pub fn sk_typeface(&self) -> &Typeface {
        &self.sk_typeface
    }

    /// Creates a Skia font of the given size with the simulations of the
    /// typeface applied.
    pub fn create_sk_font(&self, size: f32) -> Font {
        let skew_x = if self.font_simulations.contains(FontSimulations::Oblique) { -0.3 } else { 0.0 };

        let mut font = Font::from_typeface_with_params(self.sk_typeface.clone(), size, 1.0, skew_x);
        font.set_linear_metrics(true);
        font.set_embolden(self.font_simulations.contains(FontSimulations::Bold));
        font
    }

    /// The glyph bounds the glyph runs of this typeface measured, by size.
    pub(crate) fn glyph_bounds(&self) -> &GlyphBoundsCache {
        &self.glyph_bounds
    }

    /// Recovers the backend typeface behind a platform typeface (C#'s
    /// `PlatformTypeface as SkiaTypeface`).
    ///
    /// Returns `None` for typefaces created by another backend.
    pub fn try_get(platform_typeface: &dyn IPlatformTypeface) -> Option<&SkiaTypeface> {
        platform_typeface.as_any().downcast_ref::<SkiaTypeface>()
    }
}

impl IFontMemory for SkiaTypeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        let data = self.sk_typeface.copy_table_data(tag.value())?;

        Some(ReadOnlyMemory::from_slice(data.as_bytes()))
    }

    fn dispose(&self) {
        // The Skia typeface is reference counted and released with the last
        // handle to this object.
    }
}

impl IPlatformTypeface for SkiaTypeface {
    fn family_name(&self) -> String {
        self.sk_typeface.family_name()
    }

    fn weight(&self) -> FontWeight {
        self.weight
    }

    fn style(&self) -> FontStyle {
        self.style
    }

    fn stretch(&self) -> FontStretch {
        self.stretch
    }

    fn font_simulations(&self) -> FontSimulations {
        self.font_simulations
    }

    /// The position Skia reports for the typeface (`SkTypeface::getVariationDesignPosition`): a
    /// system font of Core Text or DirectWrite at a weight is an instance of a variable font.
    fn variation_settings(&self) -> FontVariationSettings {
        match self.sk_typeface.variation_design_position() {
            Some(coordinates) if !coordinates.is_empty() => FontVariationSettings::new(
                coordinates
                    .iter()
                    .filter(|coordinate| !coordinate.value.is_nan())
                    .map(|coordinate| FontVariation::new(OpenTypeTag::new(*coordinate.axis), coordinate.value as f64)),
            ),
            _ => FontVariationSettings::empty(),
        }
    }

    /// A clone of the Skia typeface at the position (`SkTypeface::makeClone` with a variation
    /// design position): the axes of `settings` take their values, the others the values of this
    /// typeface. The optical size of a text is chosen with it (#859).
    fn with_variation_settings(&self, settings: &FontVariationSettings) -> Option<Rc<dyn IPlatformTypeface>> {
        use skia_safe::font_arguments::variation_position::Coordinate;
        use skia_safe::font_arguments::VariationPosition;
        use skia_safe::{FontArguments, FourByteTag};

        let mut coordinates: Vec<Coordinate> = self.sk_typeface.variation_design_position().unwrap_or_default();
        for variation in settings.variations() {
            let axis = FourByteTag::new(variation.tag.value());
            let value = variation.value as f32;
            match coordinates.iter_mut().find(|coordinate| coordinate.axis == axis) {
                Some(coordinate) => coordinate.value = value,
                None => coordinates.push(Coordinate { axis, value }),
            }
        }
        let position = VariationPosition { coordinates: &coordinates };
        let arguments = FontArguments::new().set_variation_design_position(position);
        let typeface = self.sk_typeface.clone_with_arguments(&arguments)?;
        let typeface: Rc<dyn IPlatformTypeface> = SkiaTypeface::new(typeface, self.font_simulations);
        Some(typeface)
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let (bytes, _) = self.sk_typeface.to_font_bytes()?;

        Some(Box::new(Cursor::new(bytes)))
    }

}
