use crate::glyph_bounds_cache::GlyphBoundsCache;
use crate::skia_sharp_extensions::slant_to_font_style;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{
    FontSimulations, FontStretch, FontStyle, FontVariation, FontVariationAxis, FontVariationDesign,
    FontVariationSettings, FontWeight, IFontMemory, IPlatformTypeface,
};
use ferroui_base::utilities::ReadOnlyMemory;
use skia_safe::{Font, Typeface};
use std::any::Any;
use std::io::{Cursor, Read};
use std::rc::Rc;

/// A platform typeface over a Skia typeface.
pub struct SkiaTypeface {
    sk_typeface: Typeface,
    /// The simulations applied when a font is created from the typeface.
    font_simulations: FontSimulations,
    /// The simulations the platform applied inside the typeface (DirectWrite, fontconfig):
    /// reported, never applied again (upstream 80659c4c3e).
    platform_simulations: FontSimulations,
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
        Self::matched(typeface, font_simulations, FontSimulations::None, None)
    }

    /// Wraps a Skia typeface that a font manager matched to a request (upstream 80659c4c3e):
    /// `font_simulations` are applied, `platform_simulations` were applied by the platform
    /// inside the typeface and are only reported. With the `requested` weight and style, a
    /// simulated face reports the requested weight (bold) or style (oblique) for what is
    /// simulated; the stretch is always the face's own.
    pub fn matched(
        typeface: Typeface,
        font_simulations: FontSimulations,
        platform_simulations: FontSimulations,
        requested: Option<(FontWeight, FontStyle)>,
    ) -> Rc<Self> {
        let font_style = typeface.font_style();
        let mut weight = FontWeight(*font_style.weight());
        let mut style = slant_to_font_style(font_style.slant());
        if let Some((requested_weight, requested_style)) = requested {
            if font_simulations.contains(FontSimulations::Bold) && requested_weight.0 > weight.0 {
                weight = requested_weight;
            }
            if font_simulations.contains(FontSimulations::Oblique) && requested_style != FontStyle::Normal {
                style = requested_style;
            }
        }

        Rc::new(Self {
            weight,
            style,
            stretch: FontStretch::from_i32(*font_style.width()).unwrap_or(FontStretch::Normal),
            sk_typeface: typeface,
            font_simulations,
            platform_simulations,
            glyph_bounds: GlyphBoundsCache::default(),
        })
    }

    /// The simulations the platform applied inside the typeface.
    pub fn platform_simulations(&self) -> FontSimulations {
        self.platform_simulations
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

    /// The simulations of the face: those applied when a font is created, and those the
    /// platform applied inside the typeface.
    fn font_simulations(&self) -> FontSimulations {
        self.font_simulations | self.platform_simulations
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

    /// The axes Skia reports (`SkTypeface::getVariationDesignParameters`). Skia reads neither the
    /// names of the axes nor the named instances: those are empty here.
    fn variation_design(&self) -> FontVariationDesign {
        let axes = self.sk_typeface.variation_design_parameters().unwrap_or_default();
        FontVariationDesign {
            axes: axes
                .iter()
                .map(|axis| FontVariationAxis {
                    tag: OpenTypeTag::new(*axis.tag),
                    name: None,
                    min_value: axis.min as f64,
                    default_value: axis.def as f64,
                    max_value: axis.max as f64,
                    is_hidden: axis.is_hidden(),
                })
                .collect(),
            named_instances: Vec::new(),
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
        let typeface: Rc<dyn IPlatformTypeface> =
            SkiaTypeface::matched(typeface, self.font_simulations, self.platform_simulations, None);
        Some(typeface)
    }

    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        let (bytes, _) = self.sk_typeface.to_font_bytes()?;

        Some(Box::new(Cursor::new(bytes)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
