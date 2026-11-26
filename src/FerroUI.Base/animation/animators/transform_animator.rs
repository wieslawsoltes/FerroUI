use crate::animation::animators::{apply_with_visual, Animator, AnimatorBase, DoubleAnimator};
use crate::animation::{Animatable, Animation, AnimationError, IClock};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::transformation::TransformOperations;
use crate::media::{
    ITransform, Rotate3DTransform, RotateTransform, ScaleTransform, SkewTransform, Transform,
    TransformGroup, TranslateTransform,
};
use crate::reactive::{Disposable, IDisposable, IObservable};
use crate::{FerroProperty, Ref, StyledProperty, TypeInfo, Visual};
use std::rc::Rc;

/// Animator that handles `f64` properties of [`Transform`]s: redirects the
/// animation to the matching transform in the render transform of the
/// animated visual, creating a default render transform if there is none.
#[derive(Default)]
pub struct TransformAnimator {
    base: AnimatorBase,
}

impl TransformAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the underlying component, including defaults from the same
    /// transform group that playback would install on a bare visual.
    pub fn preview_base_value(
        control: &Animatable,
        property: &'static FerroProperty,
    ) -> Option<f64> {
        let target = preview_component_target(control, property)?;
        let property = property.as_styled::<f64>()?;
        Some(component_base(&target, property))
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

fn component_base(target: &Animatable, property: &'static StyledProperty<f64>) -> f64 {
    target.get_base_value(property).unwrap_or_else(|| {
        // An animation-only value has no stored base. The transform still
        // uses the property's default as the neutral value at a missing key.
        if property.as_property().inherits() {
            target.get_value(property)
        } else {
            property.get_default_value(target.get_type())
        }
    })
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

        let property = this.base.try_styled_property::<f64>()?.as_property();
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
                this,
                animation,
                &target,
                clock.or_else(|| control.clock()),
                obs_match,
                on_complete,
                should_pause_on_invisible,
                Some(ctrl.to_ref()),
            )));
        }

        let message = format!(
            "{} has no {} render transform for {}.",
            control.get_type(),
            property.owner_type(),
            property.name(),
        );
        if animation.is_preview() {
            return Err(AnimationError(message));
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::ANIMATIONS) {
            logger.log(None, &message);
        }

        Ok(None)
    }

    fn interpolate(&self, progress: f64, old_value: &f64, new_value: &f64) -> f64 {
        DoubleAnimator::interpolate_core(progress, old_value, new_value)
    }

    fn neutral_value(&self, target: &Animatable) -> f64 {
        component_base(target, self.base.styled_property())
    }

    fn preview_target(&self, control: &Animatable) -> Option<Ref<Animatable>> {
        preview_component_target(control, self.base.property()?)
    }
}
