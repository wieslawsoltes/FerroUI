//! Ported from `Animations/FluidMoveAnimation.cs`.

use crate::argument_out_of_range::throw_if_less_than;
use crate::AnimationRunner;
use ferroui_base::animation::{Animatable, Animation, Cue, FillMode, IAnimationSetter, KeyFrame, TimeSpan};
use ferroui_base::media::{ITransform, TranslateTransform};
use ferroui_base::styling::Setter;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::rc::Rc;

/// Creates and runs the translation animation used for fluid layout movement.
pub struct FluidMoveAnimation;

impl FluidMoveAnimation {
    /// Creates an animation from a previous layout offset (`offset_x`, `offset_y`) back to the
    /// current position, over `duration`.
    ///
    /// # Panics
    ///
    /// When `duration` is negative (the `ArgumentOutOfRangeException` of the reference).
    #[track_caller]
    pub fn create(offset_x: f64, offset_y: f64, duration: TimeSpan) -> Ref<Animation> {
        throw_if_less_than(duration, TimeSpan::ZERO, "duration");

        let animation = Animation::new();
        animation.set_duration(duration);
        animation.set_fill_mode(FillMode::Forward);
        let children = animation.children();
        let from: [Rc<dyn IAnimationSetter>; 2] = [
            Setter::new(TranslateTransform::x_property(), offset_x),
            Setter::new(TranslateTransform::y_property(), offset_y),
        ];
        children.add(KeyFrame::with_cue(Cue::new(0.0), from));
        let to: [Rc<dyn IAnimationSetter>; 2] = [
            Setter::new(TranslateTransform::x_property(), 0.0),
            Setter::new(TranslateTransform::y_property(), 0.0),
        ];
        children.add(KeyFrame::with_cue(Cue::new(1.0), to));
        animation
    }

    /// Creates and starts a fluid movement animation on a control: `target` is the target
    /// control, `offset_x` and `offset_y` the previous offsets, `duration` the animation duration.
    #[track_caller]
    pub fn run(target: &Control, offset_x: f64, offset_y: f64, duration: TimeSpan) {
        Self::prepare_transform(target, offset_x, offset_y);
        let animation = Self::create(offset_x, offset_y, duration);
        let animatable: &Animatable = target;
        AnimationRunner::try_run(Some(&animation), Some(animatable));
    }

    /// Prepares a control's translation transform and starts a fluid movement animation. Returns
    /// `true` when the target was prepared and the animation was started; otherwise, `false`.
    #[track_caller]
    pub fn try_run(target: Option<&Control>, offset_x: f64, offset_y: f64, duration: TimeSpan) -> bool {
        let Some(target) = target else {
            return false;
        };

        Self::run(target, offset_x, offset_y, duration);
        true
    }

    fn prepare_transform(target: &Control, offset_x: f64, offset_y: f64) {
        let existing = target.render_transform().and_then(|transform| {
            transform.as_object().and_then(|object| object.downcast_ref::<TranslateTransform>()).map(|t| t.to_ref())
        });
        let transform = match existing {
            Some(transform) => transform,
            None => {
                let transform = TranslateTransform::new();
                let render_transform: Rc<dyn ITransform> = (&transform).into();
                target.set_render_transform(Some(render_transform));
                transform
            }
        };

        transform.set_x(offset_x);
        transform.set_y(offset_y);
    }
}
