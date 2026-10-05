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

    /// The stop as a composition gradient stop, if it is one (the `is
    /// CompositionGradientStop` test of upstream).
    fn as_composition_gradient_stop(&self) -> Option<&crate::rendering::composition::CompositionGradientStop> {
        None
    }

    /// Whether the stop is an [`ImmutableGradientStop`](crate::media::immutable::ImmutableGradientStop)
    /// (the `as ImmutableGradientStop` test of upstream).
    fn is_immutable_gradient_stop(&self) -> bool {
        false
    }
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
