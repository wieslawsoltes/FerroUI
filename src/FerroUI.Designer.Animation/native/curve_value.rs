use crate::AnimationValue;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::{BoxedValue, CornerRadius, Point, Rect, Size, Thickness, Vector};
use std::rc::Rc;

impl From<Color> for AnimationValue {
    fn from(color: Color) -> Self {
        Self::from_argb8([color.a, color.r, color.g, color.b])
    }
}

/// Convert the native animator's result to the same types used by authored tracks.
pub(super) fn value(sample: &BoxedValue) -> Option<AnimationValue> {
    let native = sample.as_ref();
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
        let solid = brush.as_ref()?.as_solid_color_brush()?;
        return Some(solid.color().into());
    }
    if let Some(&Point { x, y }) = native.downcast_ref::<Point>() {
        return Some(AnimationValue::Point { x, y });
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
