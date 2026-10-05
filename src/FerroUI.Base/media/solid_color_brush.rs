use crate::media::ref_adapter::RefAdapter;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{Brush, BrushImpl, BrushImplExt, Color, IImmutableBrush, ServerBrushFactory};
use crate::rendering::composition::generated::ServerCompositionSimpleSolidColorBrushProps;
use crate::rendering::composition::server::ServerCompositionSimpleSolidColorBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::utilities::FormatError;
use crate::{ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty};
use std::fmt;
use std::rc::Rc;

/// Fills an area with a solid color.
#[repr(C)]
pub struct SolidColorBrush {
    base: Brush,
}

ferro_class!(SolidColorBrush: Brush);
crate::ferro_class_info!(SolidColorBrush { new: SolidColorBrush::new });
ferro_impl_classes!(SolidColorBrush: FerroObjectImpl);

impl BrushImpl for SolidColorBrush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerCompositionSimpleSolidColorBrush::new(c))
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        ServerCompositionSimpleSolidColorBrushProps::serialize_all_changes(writer, this.color());
    }
}

crate::ferro_properties! { impl SolidColorBrush {
    ferro_property!(pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<SolidColorBrush, _>("Color", Color::default())
    });
} }

impl SolidColorBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Brush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a brush with the given color.
    pub fn with_color(color: Color) -> Ref<Self> {
        Self::with_color_and_opacity(color, 1.0)
    }

    /// Creates a brush with the given color and opacity.
    pub fn with_color_and_opacity(color: Color, opacity: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result.set_opacity(opacity);
        result
    }

    /// Creates a brush with the color given as an `0xAARRGGBB` value.
    pub fn from_uint32(color: u32) -> Ref<Self> {
        Self::with_color(Color::from_uint32(color))
    }

    /// The color of the brush.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// Parses a brush string.
    pub fn parse(s: &str) -> Result<Ref<SolidColorBrush>, FormatError> {
        let brush = Brush::parse(s)?;
        let solid = brush.as_solid_color_brush().expect("parsed brushes are solid color brushes");
        Ok(match brush.as_object().and_then(|o| o.downcast_ref::<SolidColorBrush>()) {
            Some(solid) => solid.to_ref(),
            None => SolidColorBrush::with_color(solid.color()),
        })
    }

    /// Creates an immutable clone of the brush.
    pub fn to_immutable(&self) -> Rc<dyn IImmutableBrush> {
        Rc::new(ImmutableSolidColorBrush::from_brush(&RefAdapter(self.to_ref())))
    }
}

/// Writes the brush's color.
impl fmt::Display for SolidColorBrush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.color(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;
    use std::cell::Cell;

    #[test]
    fn changing_color_raises_invalidated() {
        let target = SolidColorBrush::with_color(Colors::RED);
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        target.set_color(Colors::GREEN);
        assert!(raised.get());
    }

    #[test]
    fn parse_creates_mutable_brush() {
        let brush = SolidColorBrush::parse("#40ff8844").unwrap();
        assert_eq!(Color::from_uint32(0x40ff8844), brush.color());
        assert_eq!("Red", SolidColorBrush::parse("red").unwrap().to_string());
        assert!(SolidColorBrush::parse("nope").is_err());
    }

    #[test]
    fn to_immutable_copies_values() {
        let brush = SolidColorBrush::with_color_and_opacity(Colors::RED, 0.5);
        let immutable = brush.to_immutable();
        assert_eq!(0.5, immutable.opacity());
        assert_eq!(Some(Colors::RED), immutable.as_solid_color_brush().map(|b| b.color()));
    }
}
