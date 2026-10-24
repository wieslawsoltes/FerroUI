use ferroui_base::media::text_formatting::{GenericTextRunProperties, TextRunProperties};
use ferroui_base::media::{
    BaselineAlignment, BrushExtensions, FontFeatureCollection, GlyphTypeface, IBrush,
    TextDecorationCollection, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use std::cell::OnceCell;
use std::rc::Rc;

/// Independently mutable styling owned by one visual element during construction.
/// Formatted runs receive value and collection snapshots. Native decoration
/// objects retain the framework's shared object semantics.
pub struct VisualLineElementTextRunProperties {
    typeface: Typeface,
    font_size: f64,
    decorations: Option<TextDecorationCollection>,
    foreground: Option<Rc<dyn IBrush>>,
    background: Option<Rc<dyn IBrush>>,
    baseline_alignment: BaselineAlignment,
    culture: Option<CultureInfo>,
    font_features: Option<FontFeatureCollection>,
    glyph_typeface: OnceCell<Rc<GlyphTypeface>>,
}
impl Clone for VisualLineElementTextRunProperties {
    fn clone(&self) -> Self {
        Self::new(self)
    }
}
impl VisualLineElementTextRunProperties {
    pub fn new(properties: &dyn TextRunProperties) -> Self {
        Self {
            typeface: properties.typeface().clone(),
            font_size: properties.font_rendering_em_size(),
            decorations: properties
                .text_decorations()
                .map(|value| TextDecorationCollection::from_items(value.iter())),
            foreground: properties.foreground_brush().cloned(),
            background: properties.background_brush().cloned(),
            baseline_alignment: properties.baseline_alignment(),
            culture: properties.culture_info().cloned(),
            font_features: properties
                .font_features()
                .map(|value| FontFeatureCollection::from_items(value.iter())),
            glyph_typeface: OnceCell::new(),
        }
    }
    pub fn set_typeface(&mut self, value: Typeface) {
        self.typeface = value;
        self.glyph_typeface.take();
    }
    pub fn set_font_rendering_em_size(&mut self, value: f64) {
        self.font_size = if value.is_finite() && value > 0.0 {
            value
        } else {
            1.0
        };
    }
    pub fn set_foreground_brush(&mut self, value: Option<Rc<dyn IBrush>>) {
        self.foreground = immutable(value);
    }
    pub fn set_background_brush(&mut self, value: Option<Rc<dyn IBrush>>) {
        self.background = immutable(value);
    }
    pub fn set_baseline_alignment(&mut self, value: BaselineAlignment) {
        self.baseline_alignment = value;
    }
    pub fn set_culture_info(&mut self, value: Option<CultureInfo>) {
        self.culture = value;
    }
    pub fn set_font_features(&mut self, value: Option<FontFeatureCollection>) {
        self.font_features =
            value.map(|features| FontFeatureCollection::from_items(features.iter()));
    }
    /// Adds decorations without duplicating the same native decoration object.
    /// Passing `None` clears the collection explicitly.
    pub fn set_text_decorations(&mut self, value: Option<TextDecorationCollection>) {
        let Some(value) = value else {
            self.decorations = None;
            return;
        };
        let decorations = self
            .decorations
            .get_or_insert_with(TextDecorationCollection::new);
        for decoration in value.iter() {
            if !decorations.iter().any(|current| current == decoration) {
                decorations.add(decoration);
            }
        }
    }
    pub fn snapshot(&self) -> Rc<dyn TextRunProperties> {
        Rc::new(GenericTextRunProperties::with_all(
            self.typeface.clone(),
            self.font_size,
            self.decorations
                .as_ref()
                .map(|value| TextDecorationCollection::from_items(value.iter())),
            immutable(self.foreground.clone()),
            immutable(self.background.clone()),
            self.baseline_alignment,
            self.culture.clone(),
            self.font_features
                .as_ref()
                .map(|value| FontFeatureCollection::from_items(value.iter())),
        ))
    }
}
fn immutable(brush: Option<Rc<dyn IBrush>>) -> Option<Rc<dyn IBrush>> {
    brush.map(|brush| BrushExtensions::to_immutable(&brush) as Rc<dyn IBrush>)
}
impl TextRunProperties for VisualLineElementTextRunProperties {
    fn typeface(&self) -> &Typeface {
        &self.typeface
    }
    fn font_rendering_em_size(&self) -> f64 {
        self.font_size
    }
    fn text_decorations(&self) -> Option<&TextDecorationCollection> {
        self.decorations.as_ref()
    }
    fn foreground_brush(&self) -> Option<&Rc<dyn IBrush>> {
        self.foreground.as_ref()
    }
    fn background_brush(&self) -> Option<&Rc<dyn IBrush>> {
        self.background.as_ref()
    }
    fn baseline_alignment(&self) -> BaselineAlignment {
        self.baseline_alignment
    }
    fn culture_info(&self) -> Option<&CultureInfo> {
        self.culture.as_ref()
    }
    fn font_features(&self) -> Option<&FontFeatureCollection> {
        self.font_features.as_ref()
    }
    fn cached_glyph_typeface(&self) -> Rc<GlyphTypeface> {
        self.glyph_typeface
            .get_or_init(|| self.typeface.glyph_typeface())
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::{Color, FontFamily, SolidColorBrush};

    #[test]
    fn property_clones_and_formatted_snapshots_keep_their_own_collection_and_brush_state() {
        let _scope = crate::tests::test_scope();
        let defaults = GenericTextRunProperties::with_font_size(
            Typeface::new(FontFamily::new("Test Sans")),
            12.0,
        );
        let mut first = VisualLineElementTextRunProperties::new(&defaults);
        let brush = SolidColorBrush::new();
        brush.set_color(Color::from_rgb(10, 20, 30));
        first.set_foreground_brush(Some(brush.clone().into()));
        let decorations = TextDecorationCollection::parse("Underline").unwrap();
        first.set_text_decorations(Some(decorations));
        let second = first.clone();
        let formatted = first.snapshot();
        first.set_text_decorations(Some(
            TextDecorationCollection::parse("Strikethrough").unwrap(),
        ));
        brush.set_color(Color::from_rgb(90, 80, 70));
        first.set_font_rendering_em_size(24.0);
        assert_eq!(first.text_decorations().unwrap().len(), 2);
        assert_eq!(second.text_decorations().unwrap().len(), 1);
        assert_eq!(formatted.text_decorations().unwrap().len(), 1);
        assert_eq!(formatted.font_rendering_em_size(), 12.0);
        assert_eq!(
            formatted
                .foreground_brush()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Color::from_rgb(10, 20, 30)
        );
    }
}
