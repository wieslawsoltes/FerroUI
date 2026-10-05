use std::cell::OnceCell;
use std::rc::Rc;

use crate::media::text_formatting::TextRunProperties;
use crate::media::{
    BaselineAlignment, FontFeatureCollection, GlyphTypeface, IBrush, TextDecorationCollection, Typeface,
};
use crate::utilities::CultureInfo;

/// Generic implementation of `TextRunProperties`.
pub struct GenericTextRunProperties {
    typeface: Typeface,
    font_rendering_em_size: f64,
    text_decorations: Option<TextDecorationCollection>,
    foreground_brush: Option<Rc<dyn IBrush>>,
    background_brush: Option<Rc<dyn IBrush>>,
    font_features: Option<FontFeatureCollection>,
    baseline_alignment: BaselineAlignment,
    culture_info: Option<CultureInfo>,
    cached_glyph_typeface: OnceCell<Rc<GlyphTypeface>>,
}

impl GenericTextRunProperties {
    pub(crate) const DEFAULT_FONT_RENDERING_EM_SIZE: f64 = 12.0;

    /// Creates properties for a typeface with the default em size (12) and
    /// nothing else set.
    pub fn new(typeface: Typeface) -> Self {
        Self::with_font_size(typeface, Self::DEFAULT_FONT_RENDERING_EM_SIZE)
    }

    /// Creates properties for a typeface and em size.
    pub fn with_font_size(typeface: Typeface, font_rendering_em_size: f64) -> Self {
        Self::with_all(typeface, font_rendering_em_size, None, None, None, BaselineAlignment::Baseline, None, None)
    }

    /// Creates properties with every value specified.
    #[allow(clippy::too_many_arguments)]
    pub fn with_all(
        typeface: Typeface,
        font_rendering_em_size: f64,
        text_decorations: Option<TextDecorationCollection>,
        foreground_brush: Option<Rc<dyn IBrush>>,
        background_brush: Option<Rc<dyn IBrush>>,
        baseline_alignment: BaselineAlignment,
        culture_info: Option<CultureInfo>,
        font_features: Option<FontFeatureCollection>,
    ) -> Self {
        Self {
            typeface,
            font_rendering_em_size,
            text_decorations,
            foreground_brush,
            background_brush,
            font_features,
            baseline_alignment,
            culture_info,
            cached_glyph_typeface: OnceCell::new(),
        }
    }
}

impl TextRunProperties for GenericTextRunProperties {
    fn typeface(&self) -> &Typeface {
        &self.typeface
    }

    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    fn text_decorations(&self) -> Option<&TextDecorationCollection> {
        self.text_decorations.as_ref()
    }

    fn foreground_brush(&self) -> Option<&Rc<dyn IBrush>> {
        self.foreground_brush.as_ref()
    }

    fn background_brush(&self) -> Option<&Rc<dyn IBrush>> {
        self.background_brush.as_ref()
    }

    fn font_features(&self) -> Option<&FontFeatureCollection> {
        self.font_features.as_ref()
    }

    fn baseline_alignment(&self) -> BaselineAlignment {
        self.baseline_alignment
    }

    fn culture_info(&self) -> Option<&CultureInfo> {
        self.culture_info.as_ref()
    }

    fn cached_glyph_typeface(&self) -> Rc<GlyphTypeface> {
        self.cached_glyph_typeface.get_or_init(|| self.typeface.glyph_typeface()).clone()
    }
}
