use crate::{AnimationBoxShadows, AnimationBrush, AnimationEffect, AnimationTransformList, AnimationValue, RelativeUnit};
use ferroui_base::media::{BoxShadows, Color, IBrush};
use ferroui_base::{
    BoxedValue, CornerRadius, Point, Rect, RelativePoint, RelativeScalar, Size, Thickness, Vector,
};
use std::rc::Rc;

impl From<Color> for AnimationValue {
    fn from(color: Color) -> Self {
        Self::from_argb8([color.a, color.r, color.g, color.b])
    }
}

impl From<ferroui_base::RelativeUnit> for RelativeUnit {
    fn from(unit: ferroui_base::RelativeUnit) -> Self {
        match unit {
            ferroui_base::RelativeUnit::Absolute => Self::Absolute,
            ferroui_base::RelativeUnit::Relative => Self::Relative,
        }
    }
}

/// Convert the native animator's result to the same types used by authored tracks.
pub(super) fn value(sample: &BoxedValue) -> Option<AnimationValue> {
    if let Some(effect) = AnimationEffect::from_boxed(sample) {
        return Some(AnimationValue::Effect(effect));
    }
    if let Some(list) = AnimationTransformList::from_boxed(sample) {
        return list.ok().map(AnimationValue::TransformList);
    }
    let native = sample.as_ref();
    if let Some(shadows) = native.downcast_ref::<BoxShadows>() {
        return Some(AnimationValue::BoxShadows(AnimationBoxShadows::from_native(shadows)));
    }
    if let Some(number) = native.downcast_ref::<f64>() {
        return Some(AnimationValue::Number(*number));
    }
    if let Some(number) = native.downcast_ref::<f32>() {
        return Some(AnimationValue::Number(f64::from(*number)));
    }
    if let Some(&color) = native.downcast_ref::<Color>() {
        return Some(color.into());
    }
    if let Some(brush) = native.downcast_ref::<Option<Rc<dyn IBrush>>>() {
        let brush = brush.as_deref()?;
        if let Some(solid) = brush.as_solid_color_brush() {
            return Some(solid.color().into());
        }
        return AnimationBrush::from_native(brush).ok().map(AnimationValue::Brush);
    }
    if let Some(&Point { x, y }) = native.downcast_ref::<Point>() {
        return Some(AnimationValue::Point { x, y });
    }
    if let Some(&RelativePoint { point, unit }) = native.downcast_ref::<RelativePoint>() {
        return Some(AnimationValue::RelativePoint {
            x: point.x,
            y: point.y,
            unit: unit.into(),
        });
    }
    if let Some(&RelativeScalar { scalar, unit }) = native.downcast_ref::<RelativeScalar>() {
        return Some(AnimationValue::RelativeScalar {
            scalar,
            unit: unit.into(),
        });
    }
    if let Some(&Vector { x, y }) = native.downcast_ref::<Vector>() {
        return Some(AnimationValue::Vector { x, y });
    }
    if let Some(&Size { width, height }) = native.downcast_ref::<Size>() {
        return Some(AnimationValue::Size { width, height });
    }
    if let Some(&Rect {
        x,
        y,
        width,
        height,
    }) = native.downcast_ref::<Rect>()
    {
        return Some(AnimationValue::Rect {
            x,
            y,
            width,
            height,
        });
    }
    if let Some(&Thickness {
        left,
        top,
        right,
        bottom,
    }) = native.downcast_ref::<Thickness>()
    {
        return Some(AnimationValue::Thickness {
            left,
            top,
            right,
            bottom,
        });
    }
    let &CornerRadius {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    } = native.downcast_ref::<CornerRadius>()?;
    Some(AnimationValue::CornerRadius {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    })
}
