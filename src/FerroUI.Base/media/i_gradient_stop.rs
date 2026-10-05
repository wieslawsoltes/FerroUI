use crate::media::ref_adapter::RefAdapter;
use crate::media::{Color, GradientStop};
use crate::{ObjectType, Ref, Upcast};
use std::rc::Rc;

/// Describes the location and color of a transition point in a gradient.
pub trait IGradientStop: 'static {
    /// The gradient stop color.
    fn color(&self) -> Color;

    /// The gradient stop offset.
    fn offset(&self) -> f64;
}

impl<T: ObjectType + Upcast<GradientStop>> IGradientStop for RefAdapter<T> {
    #[inline]
    fn color(&self) -> Color {
        Upcast::<GradientStop>::upcast(&*self.0).color()
    }

    #[inline]
    fn offset(&self) -> f64 {
        Upcast::<GradientStop>::upcast(&*self.0).offset()
    }
}

impl<T: ObjectType + Upcast<GradientStop>> From<Ref<T>> for Rc<dyn IGradientStop> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}
