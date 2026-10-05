use crate::animation::animators::{Animator, AnimatorBase};
use crate::animation::IAnimator;
use crate::PropertyValue;
use std::any::TypeId;
use std::rc::Rc;

/// A custom animator, as assigned to a setter with
/// [`Animation::set_animator`](crate::animation::Animation::set_animator).
pub trait ICustomAnimator: 'static {
    /// Creates the animator that plays key frames with this interpolation.
    #[doc(hidden)]
    fn create_wrapper(self: Rc<Self>) -> Rc<dyn IAnimator>;

    /// The type of the animators [`create_wrapper`](Self::create_wrapper)
    /// creates.
    #[doc(hidden)]
    fn wrapper_type(&self) -> TypeId;
}

/// The base of user-defined animators: an interpolation function for a
/// value type.
pub trait InterpolatingAnimator: 'static {
    /// The type of the values the animator interpolates, which is the value
    /// type of the properties it animates.
    type Value: PropertyValue + Default;

    /// Interpolates between two values using the specified progress.
    fn interpolate(&self, progress: f64, old_value: &Self::Value, new_value: &Self::Value) -> Self::Value;
}

impl<A: InterpolatingAnimator> ICustomAnimator for A {
    fn create_wrapper(self: Rc<Self>) -> Rc<dyn IAnimator> {
        Rc::new(AnimatorWrapper { base: AnimatorBase::new(), parent: self })
    }

    fn wrapper_type(&self) -> TypeId {
        TypeId::of::<AnimatorWrapper<A::Value>>()
    }
}

/// The animator behind every [`InterpolatingAnimator`] of value type `T`.
pub(crate) struct AnimatorWrapper<T: PropertyValue + Default> {
    base: AnimatorBase,
    parent: Rc<dyn InterpolatingAnimator<Value = T>>,
}

impl<T: PropertyValue + Default> Animator for AnimatorWrapper<T> {
    type Value = T;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn interpolate(&self, progress: f64, old_value: &T, new_value: &T) -> T {
        self.parent.interpolate(progress, old_value, new_value)
    }
}
