use crate::media::Color;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};

/// Describes the location and color of a transition point in a gradient.
#[repr(C)]
pub struct GradientStop {
    base: FerroObject,
}

ferro_class!(GradientStop: FerroObject);
crate::ferro_class_info!(GradientStop { new: GradientStop::new });
ferro_impl_classes!(GradientStop: FerroObjectImpl);

crate::ferro_properties! { impl GradientStop {
    ferro_property!(pub fn offset_property() -> StyledProperty<f64> {
        FerroProperty::register::<GradientStop, _>("Offset", 0.0)
    });

    ferro_property!(pub fn color_property() -> StyledProperty<Color> {
        FerroProperty::register::<GradientStop, _>("Color", Color::default())
    });
} }

impl GradientStop {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a gradient stop with the given color and offset.
    pub fn with_color_and_offset(color: Color, offset: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_color(color);
        result.set_offset(offset);
        result
    }

    /// The gradient stop offset.
    pub fn offset(&self) -> f64 {
        self.get_value(Self::offset_property())
    }

    pub fn set_offset(&self, value: f64) {
        self.set_value(Self::offset_property(), value)
    }

    /// The gradient stop color.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }
}
