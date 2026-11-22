use crate::animation::animators::{apply_with_visual, Animator, AnimatorBase, DoubleAnimator};
use crate::animation::{Animatable, Animation, AnimationError, IAnimator, IClock};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::transformation::TransformOperations;
use crate::media::{
    ITransform, Rotate3DTransform, RotateTransform, ScaleTransform, SkewTransform, Transform,
    TransformGroup, TranslateTransform,
};
use crate::reactive::{Disposable, IDisposable, IObservable};
use crate::{FerroProperty, Ref, TypeInfo, Visual};
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

    fn double_animator(&self, property: &'static FerroProperty) -> Rc<DoubleAnimator> {
        self.double_animator
            .borrow_mut()
            .get_or_insert_with(|| {
                let animator = Rc::new(DoubleAnimator::new());
                for keyframe in self.base.to_vec() {
                    IAnimator::add(&*animator, keyframe);
                }
                animator.base().set_property(Some(property));
                animator
            })
            .clone()
    }

    /// Read the underlying component, including defaults from the same
    /// transform group that playback would install on a bare visual.
    pub fn preview_base_value(
        control: &Animatable,
        property: &'static FerroProperty,
    ) -> Option<f64> {
        let target = preview_component_target(control, property)?;
        let property = property.as_styled::<f64>()?;
        Some(
            target
                .get_base_value(property)
                .unwrap_or_else(|| target.get_value(property)),
        )
    }
}

fn default_transform() -> Rc<dyn ITransform> {
    let group = TransformGroup::new();
    // Keep the framework's default composition order.
    let children = group.children();
    children.add(ScaleTransform::new().upcast());
    children.add(SkewTransform::new().upcast());
    children.add(RotateTransform::new().upcast());
    children.add(TranslateTransform::new().upcast());
    children.add(Rotate3DTransform::new().upcast());
    group.into()
}

fn component_target(transform: &dyn ITransform, owner: &'static TypeInfo) -> Option<Ref<Animatable>> {
    let object = transform.as_object()?;
    if object.get_type() == owner {
        return object.to_ref().cast();
    }
    if object.get_type() != TransformGroup::TYPE {
        return None;
    }
    object
        .downcast_ref::<TransformGroup>()?
        .children()
        .iter()
        .find(|child| child.get_type() == owner)
        .map(|child| child.upcast())
}

fn preview_component_target(
    control: &Animatable,
    property: &'static FerroProperty,
) -> Option<Ref<Animatable>> {
    let visual = control.downcast_ref::<Visual>()?;
    let transform = visual.render_transform().unwrap_or_else(default_transform);
    component_target(&*transform, property.owner_type())
}

/// The rejection of a target of the transform animator that is not a `Visual`
/// (`Animation::try_apply_for_preview` reports it, #567).
pub(crate) fn not_a_visual_message(control: &Animatable) -> String {
    format!("Unable to cast object of type '{}' to type 'Visual'.", control.get_type())
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
    ) -> Result<Option<Rc<dyn IDisposable>>, AnimationError> {
        let Some(ctrl) = control.downcast_ref::<Visual>() else {
            return Err(AnimationError(not_a_visual_message(control)));
        };

        let Some(property) = this.base.property() else {
            return Err(AnimationError(super::animator::NO_PROPERTY.to_owned()));
        };
        crate::animation::animation_instance::validate_timing(animation)?;

        if !Transform::TYPE.is_assignable_from(property.owner_type()) {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::ANIMATIONS) {
                logger.log(
                    None,
                    &format!(
                        "Cannot apply animation: Target property owner {} is not a Transform object.",
                        property.owner_type()
                    ),
                );
            }
            return Ok(None);
        }

        let transform = match ctrl.render_transform() {
            Some(transform) if transform.as_any().is::<TransformOperations>() => {
                return Ok(Some(Disposable::empty()));
            }
            Some(transform) => transform,
            None => {
                let transform = default_transform();
                if !animation.create_preview_transform(ctrl, transform.clone()) {
                    ctrl.set_render_transform(Some(transform.clone()));
                }
                transform
            }
        };
        animation.retain_preview_transform(ctrl, &transform);
        if let Some(target) = component_target(&*transform, property.owner_type()) {
            return Ok(Some(apply_with_visual(
                &this.double_animator(property),
                animation,
                &target,
                clock.or_else(|| control.clock()),
                obs_match,
                on_complete,
                should_pause_on_invisible,
                Some(ctrl.to_ref()),
            )));
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

        Ok(None)
    }

    fn interpolate(&self, _progress: f64, _old_value: &f64, _new_value: &f64) -> f64 {
        0.0
    }

    fn specialized_animator(&self) -> Option<Rc<dyn IAnimator>> {
        Some(self.double_animator(self.base.property()?))
    }

    fn preview_target(&self, control: &Animatable) -> Option<Ref<Animatable>> {
        preview_component_target(control, self.base.property()?)
    }
}
