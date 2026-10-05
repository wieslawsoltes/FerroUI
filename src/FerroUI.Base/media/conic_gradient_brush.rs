use crate::media::immutable::ImmutableConicGradientBrush;
use crate::media::{BrushImpl, BrushImplExt, GradientBrush, GradientBrushImpl, IImmutableBrush, ServerBrushFactory};
use crate::rendering::composition::generated::ServerCompositionSimpleConicGradientBrushProps;
use crate::rendering::composition::server::ServerCompositionSimpleConicGradientBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, RelativePoint,
    StyledProperty,
};
use std::rc::Rc;

/// Paints an area with a swept circular gradient.
#[repr(C)]
pub struct ConicGradientBrush {
    base: GradientBrush,
}

ferro_class!(ConicGradientBrush: GradientBrush);
crate::ferro_class_info!(ConicGradientBrush { new: ConicGradientBrush::new });
ferro_impl_classes!(ConicGradientBrush: FerroObjectImpl);

impl BrushImpl for ConicGradientBrush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerCompositionSimpleConicGradientBrush::new(c))
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        ServerCompositionSimpleConicGradientBrushProps::serialize_all_changes(writer, this.angle(), this.center());
    }
}

impl GradientBrushImpl for ConicGradientBrush {
    fn to_immutable(this: &Self) -> Rc<dyn IImmutableBrush> {
        Rc::new(ImmutableConicGradientBrush::from_brush(this))
    }
}

crate::ferro_properties! { impl ConicGradientBrush {
    ferro_property!(pub fn center_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<ConicGradientBrush, _>("Center", RelativePoint::CENTER)
    });

    ferro_property!(pub fn angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<ConicGradientBrush, _>("Angle", 0.0)
    });
} }

impl ConicGradientBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: GradientBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The center point of the gradient.
    pub fn center(&self) -> RelativePoint {
        self.get_value(Self::center_property())
    }

    pub fn set_center(&self, value: RelativePoint) {
        self.set_value(Self::center_property(), value)
    }

    /// The angle of the start and end of the sweep, measured from above the center point.
    pub fn angle(&self) -> f64 {
        self.get_value(Self::angle_property())
    }

    pub fn set_angle(&self, value: f64) {
        self.set_value(Self::angle_property(), value)
    }
}
