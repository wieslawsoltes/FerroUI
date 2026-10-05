use crate::animation::animators::TransformOperationsAnimator;
use crate::animation::{Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::media::transformation::TransformOperations;
use crate::media::ITransform;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles transform properties whose values are
    /// transform operation lists. Any other transform is treated as the
    /// identity.
    TransformOperationsTransition: Option<Rc<dyn ITransform>>
);

type TransformValue = Option<Rc<dyn ITransform>>;

impl Transition<TransformValue> for TransformOperationsTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: TransformValue,
        new_value: TransformValue,
    ) -> Rc<dyn IObservable<TransformValue>> {
        let old_transform = TransformOperationsAnimator::ensure_operations(&old_value);
        let new_transform = TransformOperationsAnimator::ensure_operations(&new_value);

        TransitionObservableBase::new(progress, this.easing(), move |progress| {
            let value: Rc<dyn ITransform> = TransformOperations::interpolate(&old_transform, &new_transform, progress);
            Some(value)
        })
    }
}
