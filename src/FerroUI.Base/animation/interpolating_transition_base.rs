use crate::animation::{AnimatorTransitionObservable, Transition, TransitionBase};
use crate::reactive::IObservable;
use crate::PropertyValue;
use std::rc::Rc;

/// The base for user-defined transitions that are fully described by an
/// interpolation function.
///
/// Declare the class with
/// [`ferro_transition_class!`](crate::ferro_transition_class), implement
/// this trait and implement [`Transition::do_transition`] by calling
/// [`interpolating_do_transition`].
pub trait InterpolatingTransitionBase<T: PropertyValue>: Transition<T> {
    /// Interpolates between two values using the specified progress.
    fn interpolate(this: &Self, progress: f64, from: &T, to: &T) -> T;
}

/// The implementation of [`Transition::do_transition`] for an
/// [`InterpolatingTransitionBase`].
pub fn interpolating_do_transition<C, T>(
    this: &C,
    progress: Rc<dyn IObservable<f64>>,
    old_value: T,
    new_value: T,
) -> Rc<dyn IObservable<T>>
where
    C: InterpolatingTransitionBase<T> + crate::ObjectType,
    T: PropertyValue,
{
    let base: &TransitionBase = this.upcast();
    let easing = base.easing();
    let parent = crate::FerroObject::ref_of(this);
    AnimatorTransitionObservable::create(
        move |progress, from, to| C::interpolate(&parent, progress, from, to),
        progress,
        easing,
        old_value,
        new_value,
    )
}
