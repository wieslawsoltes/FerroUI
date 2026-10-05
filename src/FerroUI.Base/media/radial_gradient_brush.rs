use crate::media::immutable::ImmutableRadialGradientBrush;
use crate::media::{BrushImpl, BrushImplExt, GradientBrush, GradientBrushImpl, IImmutableBrush, ServerBrushFactory};
use crate::rendering::composition::generated::ServerCompositionSimpleRadialGradientBrushProps;
use crate::rendering::composition::server::ServerCompositionSimpleRadialGradientBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, RelativePoint, RelativeScalar,
    StyledProperty,
};
use std::rc::Rc;

/// Paints an area with a radial gradient.
#[repr(C)]
pub struct RadialGradientBrush {
    base: GradientBrush,
}

ferro_class!(RadialGradientBrush: GradientBrush);
crate::ferro_class_info!(RadialGradientBrush { new: RadialGradientBrush::new });
ferro_impl_classes!(RadialGradientBrush: FerroObjectImpl);

impl BrushImpl for RadialGradientBrush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerCompositionSimpleRadialGradientBrush::new(c))
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        ServerCompositionSimpleRadialGradientBrushProps::serialize_all_changes(writer, this.center(),
            this.gradient_origin(),
            this.radius_x(),
            this.radius_y(),
        );
    }
}

impl GradientBrushImpl for RadialGradientBrush {
    fn to_immutable(this: &Self) -> Rc<dyn IImmutableBrush> {
        Rc::new(ImmutableRadialGradientBrush::from_brush(this))
    }
}

crate::ferro_properties! { impl RadialGradientBrush {
    ferro_property!(pub fn center_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<RadialGradientBrush, _>("Center", RelativePoint::CENTER)
    });

    ferro_property!(pub fn gradient_origin_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<RadialGradientBrush, _>("GradientOrigin", RelativePoint::CENTER)
    });

    ferro_property!(pub fn radius_x_property() -> StyledProperty<RelativeScalar> {
        FerroProperty::register::<RadialGradientBrush, _>("RadiusX", RelativeScalar::MIDDLE)
    });

    ferro_property!(pub fn radius_y_property() -> StyledProperty<RelativeScalar> {
        FerroProperty::register::<RadialGradientBrush, _>("RadiusY", RelativeScalar::MIDDLE)
    });
} }

impl RadialGradientBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: GradientBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The start point for the gradient.
    pub fn center(&self) -> RelativePoint {
        self.get_value(Self::center_property())
    }

    pub fn set_center(&self, value: RelativePoint) {
        self.set_value(Self::center_property(), value)
    }

    /// The location of the two-dimensional focal point that defines the beginning of the gradient.
    pub fn gradient_origin(&self) -> RelativePoint {
        self.get_value(Self::gradient_origin_property())
    }

    pub fn set_gradient_origin(&self, value: RelativePoint) {
        self.set_value(Self::gradient_origin_property(), value)
    }

    /// The horizontal radius of the outermost circle of the radial gradient.
    pub fn radius_x(&self) -> RelativeScalar {
        self.get_value(Self::radius_x_property())
    }

    pub fn set_radius_x(&self, value: RelativeScalar) {
        self.set_value(Self::radius_x_property(), value)
    }

    /// The vertical radius of the outermost circle of the radial gradient.
    pub fn radius_y(&self) -> RelativeScalar {
        self.get_value(Self::radius_y_property())
    }

    pub fn set_radius_y(&self, value: RelativeScalar) {
        self.set_value(Self::radius_y_property(), value)
    }
}
