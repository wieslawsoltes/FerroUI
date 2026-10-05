use crate::animation::animators::ThicknessAnimator;
use crate::animation::{AnimatorDrivenTransition, Transition, TransitionBase};
use crate::ferro_transition_class;
use crate::reactive::IObservable;
use crate::Thickness;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles properties with [`Thickness`] types.
    ThicknessTransition: Thickness
);

impl Transition<Thickness> for ThicknessTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Thickness,
        new_value: Thickness,
    ) -> Rc<dyn IObservable<Thickness>> {
        AnimatorDrivenTransition::transition::<ThicknessAnimator>(this.easing(), progress, old_value, new_value)
    }
}
