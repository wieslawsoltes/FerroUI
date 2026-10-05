use crate::animation::animators::{Animator, AnimatorBase, GradientBrushAnimator, ISolidColorBrushAnimator};
use crate::animation::{Animatable, Animation, AnimatorFactory, AnimatorKeyFrame, IAnimator, IClock};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::IBrush;
use crate::reactive::{IDisposable, IObservable};
use crate::{BoxedValue, Ref};
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

type BrushAnimatorRegistration = (Rc<dyn Fn(&dyn IBrush) -> bool>, TypeId, AnimatorFactory);

thread_local! {
    static BRUSH_ANIMATORS: RefCell<Vec<BrushAnimatorRegistration>> = const { RefCell::new(Vec::new()) };
}

/// Animator that handles all animations on properties with a brush as their
/// type: redirects them to the animator for the kind of brushes in the key
/// frames.
#[derive(Default)]
pub struct BaseBrushAnimator {
    base: AnimatorBase,
}

impl BaseBrushAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an animator for brushes for which `condition` holds; it is
    /// selected by the brush of the first key frame. Later registrations
    /// take precedence.
    pub fn register_brush_animator<TAnimator: Animator + Default>(condition: impl Fn(&dyn IBrush) -> bool + 'static) {
        let factory: AnimatorFactory = Rc::new(|| Rc::new(TAnimator::default()));
        BRUSH_ANIMATORS
            .with(|animators| animators.borrow_mut().insert(0, (Rc::new(condition), TypeId::of::<TAnimator>(), factory)));
    }

    fn brush_of(frame: &AnimatorKeyFrame) -> Option<Rc<dyn IBrush>> {
        frame.with_value(|value| {
            value.and_then(|value| value.downcast_ref::<Option<Rc<dyn IBrush>>>()).and_then(|brush| brush.clone())
        })
    }

    fn copy_key_frame(
        keyframe: &AnimatorKeyFrame,
        animator_type: TypeId,
        animator_factory: AnimatorFactory,
        value: Option<BoxedValue>,
    ) -> Ref<AnimatorKeyFrame> {
        let result = AnimatorKeyFrame::new(
            Some(animator_type),
            Some(animator_factory),
            keyframe.cue(),
            keyframe.key_spline().cloned(),
        );
        result.set_value(value);
        result.set_fill_before(keyframe.fill_before());
        result.set_fill_after(keyframe.fill_after());
        result
    }

    fn try_create_gradient_animator(&self) -> Option<Rc<dyn IAnimator>> {
        let key_frames = self.base.to_vec();
        let first_gradient =
            key_frames.iter().filter_map(|k| Self::brush_of(k)).find(|brush| brush.as_gradient_brush().is_some())?;

        let gradient_animator = Rc::new(GradientBrushAnimator::new());
        gradient_animator.base().set_property(self.base.property());
        let factory: AnimatorFactory = Rc::new(|| Rc::new(GradientBrushAnimator::new()));

        for keyframe in &key_frames {
            let brush = Self::brush_of(keyframe)?;
            let value: Option<Rc<dyn IBrush>> = if let Some(solid_color_brush) = brush.as_solid_color_brush() {
                Some(GradientBrushAnimator::convert_solid_color_brush_to_gradient(&*first_gradient, solid_color_brush))
            } else if brush.as_gradient_brush().is_some() {
                Some(brush)
            } else {
                return None;
            };
            IAnimator::add(
                &*gradient_animator,
                Self::copy_key_frame(
                    keyframe,
                    TypeId::of::<GradientBrushAnimator>(),
                    factory.clone(),
                    Some(Rc::new(value)),
                ),
            );
        }

        Some(gradient_animator)
    }

    fn try_create_solid_color_brush_animator(&self) -> Option<Rc<dyn IAnimator>> {
        let solid_color_brush_animator = Rc::new(ISolidColorBrushAnimator::new());
        solid_color_brush_animator.base().set_property(self.base.property());
        let factory: AnimatorFactory = Rc::new(|| Rc::new(ISolidColorBrushAnimator::new()));

        for keyframe in self.base.to_vec() {
            let brush = Self::brush_of(&keyframe)?;
            brush.as_solid_color_brush()?;
            IAnimator::add(
                &*solid_color_brush_animator,
                Self::copy_key_frame(
                    &keyframe,
                    TypeId::of::<ISolidColorBrushAnimator>(),
                    factory.clone(),
                    keyframe.value(),
                ),
            );
        }

        Some(solid_color_brush_animator)
    }

    fn try_create_custom_registered_animator(&self) -> Option<Rc<dyn IAnimator>> {
        let registrations = BRUSH_ANIMATORS.with(|animators| animators.borrow().clone());
        if registrations.is_empty() {
            return None;
        }
        let first_key = Self::brush_of(&self.base.get(0))?;

        for (matches, animator_type, animator_factory) in registrations {
            if !matches(&*first_key) {
                continue;
            }

            let animator = animator_factory();
            animator.set_property(self.base.property());
            for keyframe in self.base.to_vec() {
                animator.add(Self::copy_key_frame(&keyframe, animator_type, animator_factory.clone(), keyframe.value()));
            }

            return Some(animator);
        }

        None
    }
}

impl Animator for BaseBrushAnimator {
    type Value = Option<Rc<dyn IBrush>>;

    fn base(&self) -> &AnimatorBase {
        &self.base
    }

    fn apply(
        this: &Rc<Self>,
        animation: &Animation,
        control: &Animatable,
        clock: Option<Rc<dyn IClock>>,
        match_: Rc<dyn IObservable<bool>>,
        on_complete: Option<Rc<dyn Fn()>>,
        should_pause_on_invisible: bool,
    ) -> Option<Rc<dyn IDisposable>> {
        let animator = this
            .try_create_custom_registered_animator()
            .or_else(|| this.try_create_gradient_animator())
            .or_else(|| this.try_create_solid_color_brush_animator());
        if let Some(animator) = animator {
            return animator.apply(animation, control, clock, match_, on_complete, should_pause_on_invisible);
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::ANIMATIONS) {
            logger.log(None, "The animation's keyframe value types set is not supported.");
        }

        let visual_target = control.downcast_ref::<crate::Visual>().map(crate::Visual::to_ref);
        Some(crate::animation::animators::apply_with_visual(
            this,
            animation,
            control,
            clock,
            match_,
            on_complete,
            should_pause_on_invisible,
            visual_target,
        ))
    }

    fn interpolate(&self, progress: f64, old_value: &Self::Value, new_value: &Self::Value) -> Self::Value {
        if progress >= 0.5 {
            new_value.clone()
        } else {
            old_value.clone()
        }
    }
}
