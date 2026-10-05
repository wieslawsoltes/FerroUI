use crate::animation::animators::{
    Animator, AnimatorBase, ColorAnimator, DoubleAnimator, RelativePointAnimator, RelativeScalarAnimator,
};
use crate::media::immutable::{
    ImmutableConicGradientBrush, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush,
    ImmutableTransform,
};
use crate::media::transformation::TransformOperations;
use crate::media::{IBrush, IGradientBrush, IGradientStop, ISolidColorBrush, ITransform};
use std::rc::Rc;

/// Animator that handles brush properties whose key frames are gradient
/// brushes (solid color brushes are converted to gradients first).
///
/// The values are brush handles, as the animated property holds them; a
/// value that is not a gradient brush is not interpolated.
#[derive(Default)]
pub struct GradientBrushAnimator {
    base: AnimatorBase,
}

impl GradientBrushAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Interpolates between two brushes using the specified progress.
    pub fn interpolate_core(
        progress: f64,
        old_value: &Option<Rc<dyn IBrush>>,
        new_value: &Option<Rc<dyn IBrush>>,
    ) -> Option<Rc<dyn IBrush>> {
        let discrete = || if progress >= 0.5 { new_value.clone() } else { old_value.clone() };
        let (Some(old_brush), Some(new_brush)) = (old_value, new_value) else {
            return discrete();
        };
        let (Some(old_gradient), Some(new_gradient)) = (old_brush.as_gradient_brush(), new_brush.as_gradient_brush())
        else {
            return discrete();
        };

        let stops = || Self::interpolate_stops(progress, &old_gradient.gradient_stops(), &new_gradient.gradient_stops());
        let opacity = DoubleAnimator::interpolate_core(progress, &old_brush.opacity(), &new_brush.opacity());
        let transform = || Self::interpolate_transform(progress, old_brush.transform(), new_brush.transform());
        let transform_origin = RelativePointAnimator::interpolate_core(
            progress,
            &old_brush.transform_origin(),
            &new_brush.transform_origin(),
        );
        let relative_transform =
            || Self::interpolate_transform(progress, old_brush.relative_transform(), new_brush.relative_transform());

        if let (Some(old_radial), Some(new_radial)) =
            (old_brush.as_radial_gradient_brush(), new_brush.as_radial_gradient_brush())
        {
            return Some(Rc::new(ImmutableRadialGradientBrush::new(
                &stops(),
                opacity,
                transform(),
                Some(transform_origin),
                old_gradient.spread_method(),
                Some(RelativePointAnimator::interpolate_core(progress, &old_radial.center(), &new_radial.center())),
                Some(RelativePointAnimator::interpolate_core(
                    progress,
                    &old_radial.gradient_origin(),
                    &new_radial.gradient_origin(),
                )),
                Some(RelativeScalarAnimator::interpolate_core(progress, &old_radial.radius_x(), &new_radial.radius_x())),
                Some(RelativeScalarAnimator::interpolate_core(progress, &old_radial.radius_y(), &new_radial.radius_y())),
                relative_transform(),
            )));
        }

        if let (Some(old_conic), Some(new_conic)) =
            (old_brush.as_conic_gradient_brush(), new_brush.as_conic_gradient_brush())
        {
            return Some(Rc::new(ImmutableConicGradientBrush::new(
                &stops(),
                opacity,
                transform(),
                Some(transform_origin),
                old_gradient.spread_method(),
                Some(RelativePointAnimator::interpolate_core(progress, &old_conic.center(), &new_conic.center())),
                DoubleAnimator::interpolate_core(progress, &old_conic.angle(), &new_conic.angle()),
                relative_transform(),
            )));
        }

        if let (Some(old_linear), Some(new_linear)) =
            (old_brush.as_linear_gradient_brush(), new_brush.as_linear_gradient_brush())
        {
            return Some(Rc::new(ImmutableLinearGradientBrush::new(
                &stops(),
                opacity,
                transform(),
                Some(transform_origin),
                old_gradient.spread_method(),
                Some(RelativePointAnimator::interpolate_core(
                    progress,
                    &old_linear.start_point(),
                    &new_linear.start_point(),
                )),
                Some(RelativePointAnimator::interpolate_core(progress, &old_linear.end_point(), &new_linear.end_point())),
                relative_transform(),
            )));
        }

        discrete()
    }

    fn interpolate_transform(
        progress: f64,
        old_transform: Option<Rc<dyn ITransform>>,
        new_transform: Option<Rc<dyn ITransform>>,
    ) -> Option<Rc<ImmutableTransform>> {
        if let (Some(old_transform), Some(new_transform)) = (&old_transform, &new_transform) {
            if let (Some(old_operations), Some(new_operations)) = (
                old_transform.as_any().downcast_ref::<TransformOperations>(),
                new_transform.as_any().downcast_ref::<TransformOperations>(),
            ) {
                let old_operations = Self::copy_operations(old_operations);
                let new_operations = Self::copy_operations(new_operations);
                return Some(Rc::new(ImmutableTransform::new(
                    TransformOperations::interpolate(&old_operations, &new_operations, progress).value(),
                )));
            }
        }

        old_transform.map(|old_transform| Rc::new(ImmutableTransform::new(old_transform.value())))
    }

    /// The interpolation works on shared operation lists; a brush hands its
    /// transform out as an interface handle.
    fn copy_operations(operations: &TransformOperations) -> Rc<TransformOperations> {
        let mut builder = TransformOperations::create_builder(operations.operations().len());
        for operation in operations.operations() {
            builder.append(operation.clone());
        }
        builder.build()
    }

    fn interpolate_stops(
        progress: f64,
        old_value: &[Rc<dyn IGradientStop>],
        new_value: &[Rc<dyn IGradientStop>],
    ) -> Vec<ImmutableGradientStop> {
        let result_count = old_value.len().max(new_value.len());
        let mut stops = Vec::with_capacity(result_count);

        let mut old_index = 0;
        let mut new_index = 0;
        for _ in 0..result_count {
            stops.push(ImmutableGradientStop::new(
                DoubleAnimator::interpolate_core(progress, &old_value[old_index].offset(), &new_value[new_index].offset()),
                ColorAnimator::interpolate_core(progress, &old_value[old_index].color(), &new_value[new_index].color()),
            ));

            if old_index < old_value.len() - 1 {
                old_index += 1;
            }

            if new_index < new_value.len() - 1 {
                new_index += 1;
            }
        }

        stops
    }

    /// Creates a gradient of the kind of `gradient_brush` whose stops all
    /// have the color of `solid_color_brush`.
    pub(crate) fn convert_solid_color_brush_to_gradient(
        gradient_brush: &dyn IBrush,
        solid_color_brush: &dyn ISolidColorBrush,
    ) -> Rc<dyn IBrush> {
        fn create_stops_from_solid_color_brush(
            solid_color_brush: &dyn ISolidColorBrush,
            base_stops: &[Rc<dyn IGradientStop>],
        ) -> Vec<ImmutableGradientStop> {
            base_stops.iter().map(|stop| ImmutableGradientStop::new(stop.offset(), solid_color_brush.color())).collect()
        }

        let transform =
            || gradient_brush.transform().map(|transform| Rc::new(ImmutableTransform::new(transform.value())));

        if let Some(old_radial) = gradient_brush.as_radial_gradient_brush() {
            return Rc::new(ImmutableRadialGradientBrush::new(
                &create_stops_from_solid_color_brush(solid_color_brush, &old_radial.gradient_stops()),
                solid_color_brush.opacity(),
                transform(),
                Some(old_radial.transform_origin()),
                old_radial.spread_method(),
                Some(old_radial.center()),
                Some(old_radial.gradient_origin()),
                Some(old_radial.radius_x()),
                Some(old_radial.radius_y()),
                None,
            ));
        }

        if let Some(old_conic) = gradient_brush.as_conic_gradient_brush() {
            return Rc::new(ImmutableConicGradientBrush::new(
                &create_stops_from_solid_color_brush(solid_color_brush, &old_conic.gradient_stops()),
                solid_color_brush.opacity(),
                transform(),
                Some(old_conic.transform_origin()),
                old_conic.spread_method(),
                Some(old_conic.center()),
                old_conic.angle(),
                None,
            ));
        }

        if let Some(old_linear) = gradient_brush.as_linear_gradient_brush() {
            return Rc::new(ImmutableLinearGradientBrush::new(
                &create_stops_from_solid_color_brush(solid_color_brush, &old_linear.gradient_stops()),
                solid_color_brush.opacity(),
                transform(),
                Some(old_linear.transform_origin()),
                old_linear.spread_method(),
                Some(old_linear.start_point()),
                Some(old_linear.end_point()),
                None,
            ));
        }

        panic!("Gradient of this type is not supported");
    }
}

impl Animator for GradientBrushAnimator {
    type Value = Option<Rc<dyn IBrush>>;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(&self, progress: f64, old_value: &Self::Value, new_value: &Self::Value) -> Self::Value {
        Self::interpolate_core(progress, old_value, new_value)
    }
}

#[allow(dead_code)]
fn _assert_gradient(_: &dyn IGradientBrush) {}
