use std::rc::Rc;

use crate::media::text_formatting::{ITextDrawingSink, TextMetrics};
use crate::media::{
    DashStyle, GlyphRun, IBrush, IPen, MediaCollection, Pen, PenLineCap, PenLineJoin, TextDecorationLocation,
    TextDecorationUnit,
};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Point,
    Ref, StyledProperty,
};

/// Represents a text decoration, which is a visual ornamentation that is
/// added to text (such as an underline).
#[repr(C)]
pub struct TextDecoration {
    base: FerroObject,
}

ferro_class!(TextDecoration: FerroObject);
crate::ferro_class_info!(TextDecoration { new: TextDecoration::new });
ferro_impl_classes!(TextDecoration: FerroObjectImpl);

crate::ferro_properties! { impl TextDecoration {
    ferro_property!(
        /// Defines the `Location` property.
        pub fn location_property() -> StyledProperty<TextDecorationLocation> {
            FerroProperty::register::<TextDecoration, _>("Location", TextDecorationLocation::Underline)
        }
    );

    ferro_property!(
        /// Defines the `Stroke` property.
        pub fn stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextDecoration, _>("Stroke", None)
        }
    );

    ferro_property!(
        /// Defines the `StrokeThicknessUnit` property.
        pub fn stroke_thickness_unit_property() -> StyledProperty<TextDecorationUnit> {
            FerroProperty::register::<TextDecoration, _>("StrokeThicknessUnit", TextDecorationUnit::FontRecommended)
        }
    );

    ferro_property!(
        /// Defines the `StrokeDashArray` property.
        pub fn stroke_dash_array_property() -> StyledProperty<Option<MediaCollection<f64>>> {
            FerroProperty::register::<TextDecoration, _>("StrokeDashArray", None)
        }
    );

    ferro_property!(
        /// Defines the `StrokeDashOffset` property.
        pub fn stroke_dash_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<TextDecoration, _>("StrokeDashOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `StrokeThickness` property.
        pub fn stroke_thickness_property() -> StyledProperty<f64> {
            FerroProperty::register::<TextDecoration, _>("StrokeThickness", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `StrokeLineCap` property.
        pub fn stroke_line_cap_property() -> StyledProperty<PenLineCap> {
            FerroProperty::register::<TextDecoration, _>("StrokeLineCap", PenLineCap::Flat)
        }
    );

    ferro_property!(
        /// Defines the `StrokeOffset` property.
        pub fn stroke_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<TextDecoration, _>("StrokeOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `StrokeOffsetUnit` property.
        pub fn stroke_offset_unit_property() -> StyledProperty<TextDecorationUnit> {
            FerroProperty::register::<TextDecoration, _>("StrokeOffsetUnit", TextDecorationUnit::FontRecommended)
        }
    );
} }

impl TextDecoration {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates a text decoration with default values (an underline).
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the location.
    pub fn location(&self) -> TextDecorationLocation {
        self.get_value(Self::location_property())
    }

    /// Sets the location.
    pub fn set_location(&self, value: TextDecorationLocation) {
        self.set_value(Self::location_property(), value)
    }

    /// Gets the brush that specifies how the text decoration is painted.
    pub fn stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::stroke_property())
    }

    /// Sets the brush that specifies how the text decoration is painted.
    pub fn set_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::stroke_property(), value)
    }

    /// Gets the units in which the thickness of the text decoration is expressed.
    pub fn stroke_thickness_unit(&self) -> TextDecorationUnit {
        self.get_value(Self::stroke_thickness_unit_property())
    }

    /// Sets the units in which the thickness of the text decoration is expressed.
    pub fn set_stroke_thickness_unit(&self, value: TextDecorationUnit) {
        self.set_value(Self::stroke_thickness_unit_property(), value)
    }

    /// Gets a collection of values that indicate the pattern of dashes and
    /// gaps that is used to draw the text decoration.
    pub fn stroke_dash_array(&self) -> Option<MediaCollection<f64>> {
        self.get_value(Self::stroke_dash_array_property())
    }

    /// Sets a collection of values that indicate the pattern of dashes and
    /// gaps that is used to draw the text decoration.
    pub fn set_stroke_dash_array(&self, value: Option<MediaCollection<f64>>) {
        self.set_value(Self::stroke_dash_array_property(), value)
    }

    /// Gets a value that specifies the distance within the dash pattern where a dash begins.
    pub fn stroke_dash_offset(&self) -> f64 {
        self.get_value(Self::stroke_dash_offset_property())
    }

    /// Sets a value that specifies the distance within the dash pattern where a dash begins.
    pub fn set_stroke_dash_offset(&self, value: f64) {
        self.set_value(Self::stroke_dash_offset_property(), value)
    }

    /// Gets the thickness of the text decoration.
    pub fn stroke_thickness(&self) -> f64 {
        self.get_value(Self::stroke_thickness_property())
    }

    /// Sets the thickness of the text decoration.
    pub fn set_stroke_thickness(&self, value: f64) {
        self.set_value(Self::stroke_thickness_property(), value)
    }

    /// Gets a value that describes the shape at the ends of a line.
    pub fn stroke_line_cap(&self) -> PenLineCap {
        self.get_value(Self::stroke_line_cap_property())
    }

    /// Sets a value that describes the shape at the ends of a line.
    pub fn set_stroke_line_cap(&self, value: PenLineCap) {
        self.set_value(Self::stroke_line_cap_property(), value)
    }

    /// The stroke's offset.
    pub fn stroke_offset(&self) -> f64 {
        self.get_value(Self::stroke_offset_property())
    }

    /// Sets the stroke's offset.
    pub fn set_stroke_offset(&self, value: f64) {
        self.set_value(Self::stroke_offset_property(), value)
    }

    /// Gets the units in which the offset value is expressed.
    pub fn stroke_offset_unit(&self) -> TextDecorationUnit {
        self.get_value(Self::stroke_offset_unit_property())
    }

    /// Sets the units in which the offset value is expressed.
    pub fn set_stroke_offset_unit(&self, value: TextDecorationUnit) {
        self.set_value(Self::stroke_offset_unit_property(), value)
    }

    /// Draws the text decoration at given origin.
    ///
    /// * `drawing_context` — the drawing context.
    /// * `glyph_run` — the decorated run.
    /// * `text_metrics` — the font metrics of the decorated run.
    /// * `default_brush` — the default brush that is used to draw the decoration.
    pub(crate) fn draw(
        &self,
        drawing_context: &mut dyn ITextDrawingSink,
        glyph_run: &GlyphRun,
        text_metrics: &TextMetrics,
        default_brush: &Rc<dyn IBrush>,
    ) {
        let baseline_origin = glyph_run.baseline_origin();
        let mut thickness = self.stroke_thickness();
        let location = self.location();

        match self.stroke_thickness_unit() {
            TextDecorationUnit::FontRecommended => match location {
                TextDecorationLocation::Underline => thickness = text_metrics.underline_thickness,
                TextDecorationLocation::Strikethrough => thickness = text_metrics.strikethrough_thickness,
                _ => {}
            },
            TextDecorationUnit::FontRenderingEmSize => {
                thickness *= text_metrics.font_rendering_em_size;
            }
            TextDecorationUnit::Pixel => {}
        }

        let mut origin = baseline_origin;

        match location {
            TextDecorationLocation::Overline => origin.y += text_metrics.ascent,
            TextDecorationLocation::Strikethrough => origin.y += text_metrics.strikethrough_position,
            TextDecorationLocation::Underline => origin.y += text_metrics.underline_position,
            TextDecorationLocation::Baseline => {}
        }

        match self.stroke_offset_unit() {
            TextDecorationUnit::FontRenderingEmSize => {
                origin.y += self.stroke_offset() * text_metrics.font_rendering_em_size;
            }
            TextDecorationUnit::Pixel => origin.y += self.stroke_offset(),
            TextDecorationUnit::FontRecommended => {}
        }

        let dashes = self.stroke_dash_array().map(|dashes| dashes.to_vec());

        let pen: Rc<dyn IPen> = Pen::with_all(
            Some(self.stroke().unwrap_or_else(|| default_brush.clone())),
            thickness,
            Some(DashStyle::with_dashes(dashes.as_deref(), self.stroke_dash_offset()).into()),
            self.stroke_line_cap(),
            PenLineJoin::Miter,
            10.0,
        )
        .into();

        if location != TextDecorationLocation::Strikethrough {
            let offset_y = glyph_run.baseline_origin().y - origin.y;

            let intersections = glyph_run
                .get_intersections((thickness * 0.5 - offset_y) as f32, (thickness * 1.5 - offset_y) as f32);

            if !intersections.is_empty() {
                let mut last = baseline_origin.x;
                let final_pos = last + glyph_run.bounds().width;
                let mut end = last;

                let mut points = Vec::new();

                // Math is taken from chrome's source code.
                for pair in intersections.chunks_exact(2) {
                    let start = pair[0] as f64 - thickness;
                    end = pair[1] as f64 + thickness;
                    if start > last && last + text_metrics.font_rendering_em_size / 12.0 < start {
                        points.push(last);
                        points.push(start);
                    }
                    last = end;
                }

                if end < final_pos {
                    points.push(end);
                    points.push(final_pos);
                }

                for pair in points.chunks_exact(2) {
                    let a = Point::new(pair[0], origin.y);
                    let b = Point::new(pair[1], origin.y);
                    drawing_context.draw_line(&pen, a, b);
                }

                return;
            }
        }

        let p1 = origin;
        let p2 = Point::new(p1.x + glyph_run.metrics().width, p1.y);
        drawing_context.draw_line(&pen, p1, p2);
    }
}
