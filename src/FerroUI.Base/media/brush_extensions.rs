use crate::media::immutable::{ImmutableDashStyle, ImmutablePen};
use crate::media::{IBrush, IDashStyle, IImmutableBrush, IPen};
use std::rc::Rc;

/// Extension methods for brush classes.
pub struct BrushExtensions;

impl BrushExtensions {
    /// Converts a brush to an immutable brush.
    ///
    /// Returns the result of calling [`IMutableBrush::to_immutable`](crate::media::IMutableBrush::to_immutable)
    /// if the brush is mutable, otherwise the brush itself.
    pub fn to_immutable(brush: &Rc<dyn IBrush>) -> Rc<dyn IImmutableBrush> {
        if let Some(mutable) = brush.as_mutable_brush() {
            return mutable.to_immutable();
        }
        brush.clone().into_immutable_brush().expect("a brush is either mutable or immutable")
    }

    /// Converts a dash style to an immutable dash style.
    pub fn dash_style_to_immutable(style: &Rc<dyn IDashStyle>) -> Rc<ImmutableDashStyle> {
        style.clone().into_immutable_dash_style()
    }

    /// Converts a pen to an immutable pen.
    pub fn pen_to_immutable(pen: &Rc<dyn IPen>) -> Rc<ImmutablePen> {
        pen.clone().into_immutable_pen()
    }
}
