use crate::animation::animators::DoubleAnimator;
use crate::animation::easings::{Easing, LinearEasing};
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with `f64` types.
    ///
    /// A change from or to a value that is not a number (`NaN`, the automatic size of `Width`
    /// and `Height`) is not interpolated: an interpolation with `NaN` is `NaN` for the whole
    /// duration, so the new value is taken at once, when the delay has passed.
    DoubleTransition: f64
);

impl Transition<f64> for DoubleTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: f64,
        new_value: f64,
    ) -> Rc<dyn IObservable<f64>> {
        // Deviation (DEVIATIONS.md, Animation): upstream interpolates NaN, which is NaN for the
        // whole duration; the value that is not a number is taken at once.
        if old_value.is_nan() || new_value.is_nan() {
            let immediate = this.delay() == crate::animation::TimeSpan::ZERO;
            return TransitionObservableBase::new(progress, Easing::new(LinearEasing::new()), move |progress| {
                if immediate || progress > 0.0 {
                    new_value
                } else {
                    old_value
                }
            });
        }
        AnimatorDrivenTransition::transition::<DoubleAnimator>(this.easing(), progress, old_value, new_value)
    }
}
