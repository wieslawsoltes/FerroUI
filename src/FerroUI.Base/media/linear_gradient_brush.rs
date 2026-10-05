use crate::media::immutable::ImmutableLinearGradientBrush;
use crate::media::{BrushImpl, BrushImplExt, GradientBrush, GradientBrushImpl, IImmutableBrush, ServerBrushFactory};
use crate::rendering::composition::generated::ServerCompositionSimpleLinearGradientBrushProps;
use crate::rendering::composition::server::ServerCompositionSimpleLinearGradientBrush;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Ref, RelativePoint,
    StyledProperty,
};
use std::rc::Rc;

/// A brush that draws with a linear gradient.
#[repr(C)]
pub struct LinearGradientBrush {
    base: GradientBrush,
}

ferro_class!(LinearGradientBrush: GradientBrush);
crate::ferro_class_info!(LinearGradientBrush { new: LinearGradientBrush::new });
ferro_impl_classes!(LinearGradientBrush: FerroObjectImpl);

impl BrushImpl for LinearGradientBrush {
    fn factory(_this: &Self) -> Option<ServerBrushFactory> {
        Some(|c| ServerCompositionSimpleLinearGradientBrush::new(c))
    }

    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        ServerCompositionSimpleLinearGradientBrushProps::serialize_all_changes(writer, this.start_point(), this.end_point());
    }
}

impl GradientBrushImpl for LinearGradientBrush {
    fn to_immutable(this: &Self) -> Rc<dyn IImmutableBrush> {
        Rc::new(ImmutableLinearGradientBrush::from_brush(this))
    }
}

crate::ferro_properties! { impl LinearGradientBrush {
    ferro_property!(pub fn start_point_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<LinearGradientBrush, _>("StartPoint", RelativePoint::TOP_LEFT)
    });

    ferro_property!(pub fn end_point_property() -> StyledProperty<RelativePoint> {
        FerroProperty::register::<LinearGradientBrush, _>("EndPoint", RelativePoint::BOTTOM_RIGHT)
    });
} }

impl LinearGradientBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: GradientBrush::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The start point for the gradient.
    pub fn start_point(&self) -> RelativePoint {
        self.get_value(Self::start_point_property())
    }

    pub fn set_start_point(&self, value: RelativePoint) {
        self.set_value(Self::start_point_property(), value)
    }

    /// The end point for the gradient.
    pub fn end_point(&self) -> RelativePoint {
        self.get_value(Self::end_point_property())
    }

    pub fn set_end_point(&self, value: RelativePoint) {
        self.set_value(Self::end_point_property(), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{Colors, GradientSpreadMethod, GradientStop, GradientStops, IBrush};
    use crate::RelativeUnit;
    use std::cell::Cell;

    fn assert_invalidated(target: &Ref<LinearGradientBrush>, action: impl FnOnce()) {
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        let subscription = target.invalidated(move || r.set(true));
        action();
        assert!(raised.get());
        subscription.dispose();
    }

    #[test]
    fn changing_start_point_raises_invalidated() {
        let target = LinearGradientBrush::new();
        target.set_start_point(RelativePoint::default());
        assert_invalidated(&target, || target.set_start_point(RelativePoint::new(10.0, 10.0, RelativeUnit::Absolute)));
    }

    #[test]
    fn changing_end_point_raises_invalidated() {
        let target = LinearGradientBrush::new();
        target.set_end_point(RelativePoint::default());
        assert_invalidated(&target, || target.set_end_point(RelativePoint::new(10.0, 10.0, RelativeUnit::Absolute)));
    }

    #[test]
    fn changing_gradient_stops_raises_invalidated() {
        let target = LinearGradientBrush::new();
        target.set_gradient_stops(GradientStops::from_items([GradientStop::with_color_and_offset(Colors::RED, 0.0)]));
        assert_invalidated(&target, || {
            target
                .set_gradient_stops(GradientStops::from_items([GradientStop::with_color_and_offset(Colors::GREEN, 0.0)]))
        });
    }

    #[test]
    fn adding_gradient_stop_raises_invalidated() {
        let target = LinearGradientBrush::new();
        target.set_gradient_stops(GradientStops::from_items([GradientStop::with_color_and_offset(Colors::RED, 0.0)]));
        assert_invalidated(&target, || {
            target.gradient_stops().add(GradientStop::with_color_and_offset(Colors::GREEN, 1.0))
        });
    }

    #[test]
    fn changing_gradient_stop_offset_raises_invalidated() {
        let target = LinearGradientBrush::new();
        target.set_gradient_stops(GradientStops::from_items([GradientStop::with_color_and_offset(Colors::RED, 0.0)]));
        assert_invalidated(&target, || target.gradient_stops().get(0).set_offset(0.5));
    }

    #[test]
    fn replaced_gradient_stops_no_longer_invalidate() {
        let target = LinearGradientBrush::new();
        let old = target.gradient_stops();
        let stop = GradientStop::with_color_and_offset(Colors::RED, 0.0);
        old.add(stop.clone());
        target.set_gradient_stops(GradientStops::new());
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.invalidated(move || r.set(true));
        old.add(GradientStop::new());
        stop.set_offset(1.0);
        assert!(!raised.get());
    }

    #[test]
    fn defaults_and_immutable_copy() {
        let target = LinearGradientBrush::new();
        assert_eq!(RelativePoint::TOP_LEFT, target.start_point());
        assert_eq!(RelativePoint::BOTTOM_RIGHT, target.end_point());
        target.gradient_stops().add(GradientStop::with_color_and_offset(Colors::RED, 2.0));
        target.set_spread_method(GradientSpreadMethod::Reflect);
        target.set_opacity(0.25);

        let brush: Rc<dyn IBrush> = target.clone().into();
        let linear = brush.as_linear_gradient_brush().unwrap();
        assert_eq!(RelativePoint::BOTTOM_RIGHT, linear.end_point());
        assert_eq!(1, linear.gradient_stops().len());
        assert_eq!(2.0, linear.gradient_stops()[0].offset());

        let immutable = crate::media::BrushExtensions::to_immutable(&brush);
        let linear = immutable.as_linear_gradient_brush().unwrap();
        assert_eq!(0.25, linear.opacity());
        assert_eq!(GradientSpreadMethod::Reflect, linear.spread_method());
        // Offsets are clamped to [0, 1] by the immutable stop.
        assert_eq!(1.0, linear.gradient_stops()[0].offset());
        assert_eq!(Colors::RED, linear.gradient_stops()[0].color());
        assert!(immutable.as_mutable_brush().is_none());
    }
}
