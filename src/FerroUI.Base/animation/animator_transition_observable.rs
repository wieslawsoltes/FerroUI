use crate::animation::easings::Easing;
use crate::animation::TransitionObservableBase;
use crate::reactive::IObservable;
use std::rc::Rc;

/// Transition observable based on an animator's interpolation: produces
/// the values between `old_value` and `new_value`.
pub struct AnimatorTransitionObservable;

impl AnimatorTransitionObservable {
    pub fn create<T: 'static>(
        interpolate: impl Fn(f64, &T, &T) -> T + 'static,
        progress: Rc<dyn IObservable<f64>>,
        easing: Easing,
        old_value: T,
        new_value: T,
    ) -> Rc<dyn IObservable<T>> {
        TransitionObservableBase::new(progress, easing, move |progress| {
            interpolate(progress, &old_value, &new_value)
        })
    }
}
