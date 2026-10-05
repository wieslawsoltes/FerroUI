use crate::animation::animators::{apply_with_visual, Animator, AnimatorBase, DoubleAnimator};
use crate::animation::{Animatable, Animation, IAnimator, IClock};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::transformation::TransformOperations;
use crate::media::{
    ITransform, RotateTransform, Rotate3DTransform, ScaleTransform, SkewTransform, Transform, TransformGroup,
    TranslateTransform,
};
use crate::reactive::{Disposable, IDisposable, IObservable};
use crate::{Ref, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// Animator that handles `f64` properties of [`Transform`]s: redirects the
/// animation to the matching transform in the render transform of the
/// animated visual, creating a default render transform if there is none.
#[derive(Default)]
pub struct TransformAnimator {
    base: AnimatorBase,
    double_animator: RefCell<Option<Rc<DoubleAnimator>>>,
}

impl TransformAnimator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Animator for TransformAnimator {
    type Value = f64;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn apply(
        this: &Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        obs_match: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>> {
        let Some(ctrl) = control.downcast_ref::<Visual>() else {
            panic!("Unable to cast object of type '{}' to type 'Visual'.", control.get_type());
        };

        let Some(property) = this.base.property() else {
            panic!("Animator has no property specified.");
        };

        // Check if the Target Property is Transform derived.
        if Transform::TYPE.is_assignable_from(property.owner_type()) {
            if ctrl.render_transform().is_some_and(|t| t.as_any().is::<TransformOperations>()) {
                // HACK: This animator cannot reasonably animate CSS transforms at the moment.
                return Some(Disposable::empty());
            }

            let render_transform: Rc<dyn ITransform> = match ctrl.render_transform() {
                Some(render_transform) => render_transform,
                None => {
                    let normal_transform = TransformGroup::new();

                    // Add the transforms according to MS Expression Blend's
                    // default RenderTransform order.
                    let children = normal_transform.children();
                    children.add(ScaleTransform::new().upcast());
                    children.add(SkewTransform::new().upcast());
                    children.add(RotateTransform::new().upcast());
                    children.add(TranslateTransform::new().upcast());
                    children.add(Rotate3DTransform::new().upcast());

                    let render_transform: Rc<dyn ITransform> = normal_transform.into();
                    ctrl.set_render_transform(Some(render_transform.clone()));
                    render_transform
                }
            };

            let double_animator = {
                let mut double_animator = this.double_animator.borrow_mut();
                double_animator
                    .get_or_insert_with(|| {
                        let double_animator = Rc::new(DoubleAnimator::new());
                        for keyframe in this.base.to_vec() {
                            IAnimator::add(&*double_animator, keyframe);
                        }
                        double_animator.base().set_property(Some(property));
                        double_animator
                    })
                    .clone()
            };

            let visual_target = Some(ctrl.to_ref());
            let clock = clock.or_else(|| control.clock());

            if let Some(render_transform) = render_transform.as_object() {
                // It's a transform object so let's target that.
                if render_transform.get_type() == property.owner_type() {
                    let target: Ref<Animatable> =
                        render_transform.to_ref().cast().expect("a transform is an animatable");
                    return Some(apply_with_visual(
                        &double_animator,
                        animation,
                        &target,
                        clock,
                        obs_match,
                        on_complete,
                        should_pause_on_invisible,
                        visual_target,
                    ));
                }
                // It's a TransformGroup and try finding the target there.
                else if let Some(group) = render_transform.downcast_ref::<TransformGroup>() {
                    if render_transform.get_type() == TransformGroup::TYPE {
                        for transform in group.children().iter() {
                            if transform.get_type() == property.owner_type() {
                                let target: Ref<Animatable> = transform.upcast();
                                return Some(apply_with_visual(
                                    &double_animator,
                                    animation,
                                    &target,
                                    clock,
                                    obs_match,
                                    on_complete,
                                    should_pause_on_invisible,
                                    visual_target,
                                ));
                            }
                        }
                    }
                }
            }

            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::ANIMATIONS) {
                logger.log(
                    None,
                    &format!(
                        "Cannot find the appropriate transform: \"{}\" in {}.",
                        property.owner_type(),
                        control.get_type()
                    ),
                );
            }
        } else if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::ANIMATIONS) {
            logger.log(
                None,
                &format!(
                    "Cannot apply animation: Target property owner {} is not a Transform object.",
                    property.owner_type()
                ),
            );
        }

        None
    }

    fn interpolate(&self, _progress: f64, _old_value: &f64, _new_value: &f64) -> f64 {
        0.0
    }
}
