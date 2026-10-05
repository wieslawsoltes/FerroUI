use crate::animation::animators::{GradientBrushAnimator, ISolidColorBrushAnimator};
use crate::animation::{AnimatorTransitionObservable, Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::media::IBrush;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles brush properties.
    ///
    /// Only values of solid color and gradient brushes can be transitioned.
    BrushTransition: Option<Rc<dyn IBrush>>
);

type Brush = Option<Rc<dyn IBrush>>;

impl Transition<Brush> for BrushTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Brush,
        new_value: Brush,
    ) -> Rc<dyn IObservable<Brush>> {
        let easing = this.easing();
        let incompatible = |progress, old_value: Brush, new_value: Brush| -> Rc<dyn IObservable<Brush>> {
            TransitionObservableBase::new(progress, this.easing(), move |progress| {
                if progress >= 0.5 {
                    new_value.clone()
                } else {
                    old_value.clone()
                }
            })
        };

        let (Some(old_brush), Some(new_brush)) = (&old_value, &new_value) else {
            return incompatible(progress, old_value, new_value);
        };

        if old_brush.as_gradient_brush().is_some() {
            if new_brush.as_gradient_brush().is_some() {
                return AnimatorTransitionObservable::create(
                    GradientBrushAnimator::interpolate_core,
                    progress,
                    easing,
                    old_value,
                    new_value,
                );
            } else if let Some(new_solid_color_brush_to_convert) = new_brush.as_solid_color_brush() {
                let converted_solid_color_brush = GradientBrushAnimator::convert_solid_color_brush_to_gradient(
                    &**old_brush,
                    new_solid_color_brush_to_convert,
                );
                return AnimatorTransitionObservable::create(
                    GradientBrushAnimator::interpolate_core,
                    progress,
                    easing,
                    old_value,
                    Some(converted_solid_color_brush),
                );
            }
        } else if let (Some(_), Some(old_solid_color_brush_to_convert)) =
            (new_brush.as_gradient_brush(), old_brush.as_solid_color_brush())
        {
            let converted_solid_color_brush = GradientBrushAnimator::convert_solid_color_brush_to_gradient(
                &**new_brush,
                old_solid_color_brush_to_convert,
            );
            return AnimatorTransitionObservable::create(
                GradientBrushAnimator::interpolate_core,
                progress,
                easing,
                Some(converted_solid_color_brush),
                new_value,
            );
        }

        if old_brush.as_solid_color_brush().is_some() && new_brush.as_solid_color_brush().is_some() {
            return AnimatorTransitionObservable::create(
                ISolidColorBrushAnimator::interpolate_core,
                progress,
                easing,
                old_value,
                new_value,
            );
        }

        incompatible(progress, old_value, new_value)
    }
}
