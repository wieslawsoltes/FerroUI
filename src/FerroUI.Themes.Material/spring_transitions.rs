// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: MIT
//! Material property transitions with per-target, interruption-continuous spring state.
//!
//! These accept the same `Property`, `Duration`, `Delay` and `Easing` properties as the
//! built-in transitions. A Material spring easing runs in physical seconds and preserves
//! each component's incoming velocity on interruption. `Duration` is a minimum settling
//! interval, not a time stretch; the tail extends when incoming momentum requires it.
//! Other easings retain the normal duration-based behavior. Reduced motion completes on
//! the next clock notification, releases its subscription and cannot revive canceled motion.

use crate::motion::{MaterialSpringEasing, MaterialSpringSample};
use ferroui_base::animation::animators::{CornerRadiusAnimator, DoubleAnimator, ThicknessAnimator};
use ferroui_base::animation::{
    Animatable, AnimatorDrivenTransition, IClock, TimeSpan, Transition, TransitionBase,
    TransitionBaseImpl,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::{
    IDisposable, IObservable, IObserver, Observable, ObservableError,
    SingleSubscriberObservableBase,
};
use ferroui_base::{
    AnyValue, BoxedValue, CornerRadius, FerroObjectImpl, FerroProperty, PropertyValue, Ref,
    Thickness, Visual,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// A physical spring transition for scalar properties, including opacity and switch geometry.
#[repr(C)]
pub struct MaterialDoubleTransition {
    base: TransitionBase,
}
ferroui_base::ferro_class!(MaterialDoubleTransition: TransitionBase);
ferroui_base::ferro_class_info!(MaterialDoubleTransition {
    new: MaterialDoubleTransition::new
});

/// A physical spring transition with independent velocity for all four corner radii.
#[repr(C)]
pub struct MaterialCornerRadiusTransition {
    base: TransitionBase,
}
ferroui_base::ferro_class!(MaterialCornerRadiusTransition: TransitionBase);
ferroui_base::ferro_class_info!(MaterialCornerRadiusTransition {
    new: MaterialCornerRadiusTransition::new
});

/// A physical spring transition with independent velocity for all four thickness components.
#[repr(C)]
pub struct MaterialThicknessTransition {
    base: TransitionBase,
}
ferroui_base::ferro_class!(MaterialThicknessTransition: TransitionBase);
ferroui_base::ferro_class_info!(MaterialThicknessTransition {
    new: MaterialThicknessTransition::new
});

macro_rules! implement_transition {
    ($name:ident, $value:ty, $animator:ty) => {
        impl FerroObjectImpl for $name {}
        impl TransitionBaseImpl for $name {
            fn apply(
                this: &Self,
                control: &Animatable,
                clock: Rc<dyn IClock>,
                old: &BoxedValue,
                new: &BoxedValue,
            ) -> Rc<dyn IDisposable> {
                apply_spring::<Self, $value>(this, control, clock, old, new)
            }
            fn value_type(_: &Self) -> (TypeId, &'static str) {
                (TypeId::of::<$value>(), std::any::type_name::<$value>())
            }
        }
        impl Transition<$value> for $name {
            fn do_transition(
                this: &Self,
                progress: Rc<dyn IObservable<f64>>,
                old: $value,
                new: $value,
            ) -> Rc<dyn IObservable<$value>> {
                AnimatorDrivenTransition::transition::<$animator>(this.easing(), progress, old, new)
            }
        }
        impl $name {
            pub fn construct() -> Self {
                Self {
                    base: TransitionBase::construct(),
                }
            }
            pub fn new() -> Ref<Self> {
                ferroui_base::instantiate(Self::construct())
            }
            pub fn with_property(
                property: &'static FerroProperty,
                duration: TimeSpan,
            ) -> Ref<Self> {
                let result = Self::new();
                result.set_property(Some(property));
                result.set_duration(duration);
                result
            }
        }
    };
}
implement_transition!(MaterialDoubleTransition, f64, DoubleAnimator);
implement_transition!(
    MaterialCornerRadiusTransition,
    CornerRadius,
    CornerRadiusAnimator
);
implement_transition!(MaterialThicknessTransition, Thickness, ThicknessAnimator);

trait SpringValue: PropertyValue {
    const COUNT: usize;
    fn components(&self) -> [f64; 4];
    fn from_components(value: [f64; 4]) -> Self;
    fn constraint(property: &'static FerroProperty) -> Constraint;
}

#[derive(Clone, Copy)]
enum Constraint {
    None,
    Nonnegative,
    Unit,
}

impl Constraint {
    fn project(self, sample: MaterialSpringSample) -> MaterialSpringSample {
        let value = match self {
            Self::None => sample.value,
            Self::Nonnegative => sample.value.max(0.0),
            Self::Unit => sample.value.clamp(0.0, 1.0),
        };
        if value == sample.value {
            sample
        } else {
            MaterialSpringSample {
                value,
                velocity: 0.0,
            }
        }
    }
}

impl SpringValue for f64 {
    const COUNT: usize = 1;
    fn components(&self) -> [f64; 4] {
        [*self, 0.0, 0.0, 0.0]
    }
    fn from_components(value: [f64; 4]) -> Self {
        value[0]
    }
    fn constraint(property: &'static FerroProperty) -> Constraint {
        if property == Visual::opacity_property().as_property() {
            Constraint::Unit
        } else if property == Layoutable::width_property().as_property()
            || property == Layoutable::height_property().as_property()
        {
            Constraint::Nonnegative
        } else {
            Constraint::None
        }
    }
}

impl SpringValue for CornerRadius {
    const COUNT: usize = 4;
    fn components(&self) -> [f64; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
    fn from_components(v: [f64; 4]) -> Self {
        Self::new(v[0], v[1], v[2], v[3])
    }
    fn constraint(_: &'static FerroProperty) -> Constraint {
        Constraint::Nonnegative
    }
}

impl SpringValue for Thickness {
    const COUNT: usize = 4;
    fn components(&self) -> [f64; 4] {
        [self.left, self.top, self.right, self.bottom]
    }
    fn from_components(v: [f64; 4]) -> Self {
        Self::new(v[0], v[1], v[2], v[3])
    }
    fn constraint(_: &'static FerroProperty) -> Constraint {
        Constraint::None
    }
}

/// Kept by the target's transition state, never by a shared resource or the target's observer.
struct SpringFrame {
    current: [MaterialSpringSample; 4],
    last_tick: Option<TimeSpan>,
    clock_id: *const (),
    completed: bool,
}

fn apply_spring<C: Transition<T>, T: SpringValue>(
    this: &C,
    control: &Animatable,
    clock: Rc<dyn IClock>,
    old: &BoxedValue,
    new: &BoxedValue,
) -> Rc<dyn IDisposable> {
    let base: &TransitionBase = this.upcast();
    let easing = base.easing();
    let Some(spring) = (easing.as_rc().as_ref() as &dyn Any).downcast_ref::<MaterialSpringEasing>()
    else {
        // Clear any earlier spring's momentum when a caller switches to a nonphysical easing.
        let _ = control.take_transition_interruption_state::<RefCell<SpringFrame>>();
        return ferroui_base::animation::apply_transition::<C, T>(this, control, clock, old, new);
    };
    let typed = |value: &BoxedValue| -> T {
        let value: &dyn AnyValue = &**value;
        value
            .downcast_ref::<T>()
            .expect("Material transition property type matches its value")
            .clone()
    };
    let old_value = typed(old);
    let new_value = typed(new);
    let from = old_value.components();
    let target = new_value.components();
    let previous = control.take_transition_interruption_state::<RefCell<SpringFrame>>();
    if !from.into_iter().chain(target).all(f64::is_finite) {
        return ferroui_base::animation::apply_transition::<C, T>(this, control, clock, old, new);
    }
    let property = base
        .property()
        .expect("Material transition requires a property");
    let property = property
        .as_styled::<T>()
        .expect("Material transitions require a styled property");
    if spring.motion_disabled() || base.duration() <= TimeSpan::ZERO {
        return control.bind(
            property,
            Observable::return_(new_value),
            BindingPriority::Animation,
        );
    }
    let clock_id = Rc::as_ptr(&clock) as *const ();
    let mut origin = from.map(|value| MaterialSpringSample {
        value,
        velocity: 0.0,
    });
    let mut start_at = None;
    if let Some(previous) = previous {
        let previous = previous.borrow();
        if !previous.completed && previous.clock_id == clock_id {
            // A different animation or a property coercion can replace the current value.
            // Preserve only momentum belonging to the value we are actually retargeting.
            for i in 0..T::COUNT {
                if (previous.current[i].value - from[i]).abs() <= 1e-9 {
                    origin[i].velocity = previous.current[i].velocity;
                }
            }
            start_at = previous.last_tick;
        }
    }
    let spec = spring.spring_spec();
    let duration = (0..T::COUNT).fold(base.duration().total_seconds(), |duration, i| {
        duration.max(spec.settling_duration_from(
            origin[i].value - target[i],
            origin[i].velocity,
            0.001,
            0.001 * spec.stiffness().sqrt(),
        ))
    });
    let frame = Rc::new(RefCell::new(SpringFrame {
        current: origin,
        last_tick: start_at,
        clock_id,
        completed: false,
    }));
    control.report_transition_interruption_state(frame.clone());
    let constraint = T::constraint(property.as_property());
    let observable = Rc::new_cyclic(|weak| SpringObservable::<T> {
        this: weak.clone(),
        core: SingleSubscriberObservableBase::new(),
        subscription: RefCell::new(None),
        clock,
        frame,
        origin,
        target,
        start_at: Cell::new(start_at),
        duration,
        delay: base.delay().total_seconds().max(0.0),
        spring: spring.clone(),
        constraint,
        _value: PhantomData,
    });
    control.bind(property, observable, BindingPriority::Animation)
}

struct SpringObservable<T: SpringValue> {
    this: Weak<Self>,
    core: SingleSubscriberObservableBase<T>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    clock: Rc<dyn IClock>,
    frame: Rc<RefCell<SpringFrame>>,
    origin: [MaterialSpringSample; 4],
    target: [f64; 4],
    start_at: Cell<Option<TimeSpan>>,
    duration: f64,
    delay: f64,
    spring: MaterialSpringEasing,
    constraint: Constraint,
    _value: PhantomData<T>,
}

impl<T: SpringValue> SpringObservable<T> {
    fn unsubscribe(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn finish(&self) {
        {
            let mut frame = self.frame.borrow_mut();
            frame.current = self.target.map(|value| MaterialSpringSample {
                value,
                velocity: 0.0,
            });
            frame.completed = true;
        }
        self.core.publish_next(T::from_components(self.target));
        self.core.publish_completed(|| self.unsubscribe());
    }
}

impl<T: SpringValue> IObservable<T> for SpringObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.core.subscribe(observer, || {
            self.core
                .publish_next(T::from_components(self.origin.map(|sample| sample.value)));
            if !self.core.has_observer() {
                return;
            }
            let this = self
                .this
                .upgrade()
                .expect("spring observable alive during subscribe");
            let subscription = self.clock.subscribe(this);
            if self.core.has_observer() {
                *self.subscription.borrow_mut() = Some(subscription);
            } else {
                subscription.dispose();
            }
        });
        self.this
            .upgrade()
            .expect("spring observable alive during subscribe")
    }
}

impl<T: SpringValue> IObserver<TimeSpan> for SpringObservable<T> {
    fn on_next(&self, tick: TimeSpan) {
        if !self.core.has_observer() {
            return;
        }
        if self
            .frame
            .borrow()
            .last_tick
            .is_some_and(|last| tick < last)
        {
            return;
        }
        let start = self.start_at.get().unwrap_or_else(|| {
            self.start_at.set(Some(tick));
            tick
        });
        let elapsed = (tick - start).total_seconds() - self.delay;
        if self.spring.motion_disabled() || elapsed >= self.duration {
            self.finish();
            return;
        }
        let samples = std::array::from_fn(|i| {
            self.constraint.project(self.spring.spring_spec().sample(
                elapsed.max(0.0),
                self.origin[i].value,
                self.target[i],
                self.origin[i].velocity,
            ))
        });
        {
            let mut frame = self.frame.borrow_mut();
            frame.current = samples;
            frame.last_tick = Some(tick);
        }
        self.core
            .publish_next(T::from_components(samples.map(|sample| sample.value)));
    }
    fn on_completed(&self) {
        self.finish();
    }
    fn on_error(&self, error: ObservableError) {
        self.frame.borrow_mut().completed = true;
        self.core.publish_error(error, || self.unsubscribe());
    }
}

impl<T: SpringValue> IDisposable for SpringObservable<T> {
    fn dispose(&self) {
        self.core.dispose(|| self.unsubscribe());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{MaterialMotionPolicy, MaterialMotionRole, MaterialMotionScheme};
    use ferroui_base::animation::{ITransition, PlayState, Transitions};
    use ferroui_base::reactive::LightweightSubject;
    use ferroui_base::Visual;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_controls::{Border, Button, Control, Window};

    struct TestClock {
        ticks: LightweightSubject<TimeSpan>,
    }
    impl TestClock {
        fn new() -> Rc<Self> {
            Rc::new(Self {
                ticks: LightweightSubject::new(),
            })
        }
        fn tick(&self, seconds: f64) {
            self.ticks.on_next(TimeSpan::from_seconds(seconds));
        }
    }
    impl IObservable<TimeSpan> for TestClock {
        fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
            self.ticks.subscribe(observer)
        }
    }
    impl IClock for TestClock {
        fn play_state(&self) -> PlayState {
            PlayState::Run
        }
        fn set_play_state(&self, _: PlayState) {}
    }
    impl ferroui_base::animation::IGlobalClock for TestClock {}

    fn close(expected: f64, actual: f64) {
        assert!(
            (expected - actual).abs() < 1e-7,
            "expected {expected}, actual {actual}"
        );
    }

    fn width_transition(role: MaterialMotionRole) -> Ref<MaterialDoubleTransition> {
        let scheme = MaterialMotionScheme::Expressive;
        let transition = MaterialDoubleTransition::with_property(
            Layoutable::width_property().as_property(),
            scheme.duration(role, false),
        );
        transition.set_easing(scheme.easing(role, false));
        transition
    }

    fn border(clock: &Rc<TestClock>, transition: Rc<dyn ITransition>, width: f64) -> Ref<Border> {
        let border = Border::new();
        border.set_clock(Some(clock.clone()));
        border.set_width(width);
        border.set_transitions(Some(Transitions::from_items([transition])));
        border.enable_transitions();
        border
    }

    #[test]
    fn interrupted_scalar_preserves_position_and_velocity_without_losing_a_frame() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::FastSpatial;
        let spec = MaterialMotionScheme::Expressive.spring(role);
        let target = border(&clock, width_transition(role).into(), 20.0);
        target.set_width(120.0);
        clock.tick(0.0);
        clock.tick(0.04);
        let incoming = spec.sample(0.04, 20.0, 120.0, 0.0);
        close(incoming.value, target.width());
        target.set_width(40.0);
        close(incoming.value, target.width());
        clock.tick(0.041);
        close(
            spec.sample(0.001, incoming.value, 40.0, incoming.velocity)
                .value,
            target.width(),
        );
        clock.tick(2.0);
        assert_eq!(40.0, target.width());
        assert!(
            !clock.ticks.has_observers(),
            "settled springs release their clock immediately"
        );
    }

    #[test]
    fn one_shared_transition_keeps_each_controls_momentum_separate() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::DefaultSpatial;
        let spec = MaterialMotionScheme::Expressive.spring(role);
        let transition: Rc<dyn ITransition> = width_transition(role).into();
        let a = border(&clock, transition.clone(), 10.0);
        let b = border(&clock, transition, 150.0);
        a.set_width(110.0);
        b.set_width(50.0);
        clock.tick(0.0);
        clock.tick(0.03);
        let sa = spec.sample(0.03, 10.0, 110.0, 0.0);
        let sb = spec.sample(0.03, 150.0, 50.0, 0.0);
        a.set_width(70.0);
        b.set_width(90.0);
        clock.tick(0.04);
        close(
            spec.sample(0.01, sa.value, 70.0, sa.velocity).value,
            a.width(),
        );
        close(
            spec.sample(0.01, sb.value, 90.0, sb.velocity).value,
            b.width(),
        );
        a.disable_transitions();
        b.disable_transitions();
        assert!(!clock.ticks.has_observers());
    }

    #[test]
    fn four_corner_radii_retain_independent_velocity() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::DefaultEffects;
        let scheme = MaterialMotionScheme::Expressive;
        let spec = scheme.spring(role);
        let transition = MaterialCornerRadiusTransition::with_property(
            Border::corner_radius_property().as_property(),
            scheme.duration(role, false),
        );
        transition.set_easing(scheme.easing(role, false));
        let target = border(&clock, transition.into(), 20.0);
        let first = CornerRadius::new(8.0, 12.0, 20.0, 32.0);
        let second = CornerRadius::new(24.0, 4.0, 8.0, 40.0);
        target.set_corner_radius(first);
        clock.tick(0.0);
        clock.tick(0.04);
        target.set_corner_radius(second);
        clock.tick(0.05);
        for (i, actual) in target.corner_radius().components().iter().enumerate() {
            let incoming = spec.sample(0.04, 0.0, first.components()[i], 0.0);
            close(
                spec.sample(
                    0.01,
                    incoming.value,
                    second.components()[i],
                    incoming.velocity,
                )
                .value,
                *actual,
            );
        }
        target.disable_transitions();
    }

    #[test]
    fn thickness_transition_preserves_negative_margin_and_component_velocity() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::DefaultSpatial;
        let scheme = MaterialMotionScheme::Expressive;
        let spec = scheme.spring(role);
        let transition = MaterialThicknessTransition::with_property(
            Layoutable::margin_property().as_property(),
            scheme.duration(role, false),
        );
        transition.set_easing(scheme.easing(role, false));
        let target = border(&clock, transition.into(), 20.0);
        let first = Thickness::new(10.0, -8.0, 0.0, 24.0);
        let second = Thickness::new(-5.0, 8.0, 16.0, 0.0);
        target.set_margin(first);
        clock.tick(0.0);
        clock.tick(0.04);
        assert!(target.margin().top < 0.0);
        target.set_margin(second);
        clock.tick(0.05);
        for (i, actual) in target.margin().components().iter().enumerate() {
            let incoming = spec.sample(0.04, 0.0, first.components()[i], 0.0);
            close(
                spec.sample(
                    0.01,
                    incoming.value,
                    second.components()[i],
                    incoming.velocity,
                )
                .value,
                *actual,
            );
        }
        target.disable_transitions();
    }

    #[test]
    fn detachment_unsubscribes_and_reattachment_starts_without_stale_momentum() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::FastSpatial;
        let spec = MaterialMotionScheme::Expressive.spring(role);
        let target = border(&clock, width_transition(role).into(), 20.0);
        target.set_width(120.0);
        clock.tick(0.0);
        clock.tick(0.04);
        target.disable_transitions();
        assert!(!clock.ticks.has_observers());
        assert_eq!(120.0, target.width());
        target.enable_transitions();
        target.set_width(60.0);
        clock.tick(1.0);
        clock.tick(1.04);
        close(spec.sample(0.04, 120.0, 60.0, 0.0).value, target.width());
        target.disable_transitions();
    }

    #[test]
    fn reduced_motion_finishes_active_spring_and_canceled_state_never_restarts() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let role = MaterialMotionRole::FastSpatial;
        let policy = MaterialMotionPolicy::default();
        let transition = width_transition(role);
        transition.set_easing(MaterialMotionScheme::Expressive.easing_with_policy(role, &policy));
        let target = border(&clock, transition.into(), 20.0);
        target.set_width(120.0);
        clock.tick(0.0);
        clock.tick(0.03);
        policy.set_reduced_motion(true);
        policy.set_reduced_motion(false);
        clock.tick(0.04);
        assert_eq!(120.0, target.width());
        assert!(!clock.ticks.has_observers());
        target.set_width(80.0);
        assert_eq!(80.0, target.width());
        assert!(
            !clock.ticks.has_observers(),
            "canceled easing cannot restart a clock"
        );
    }

    #[test]
    fn initial_reduced_motion_never_subscribes_to_a_clock() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let transition = width_transition(MaterialMotionRole::FastSpatial);
        transition.set_duration(TimeSpan::ZERO);
        let target = border(&clock, transition.into(), 20.0);
        target.set_width(120.0);
        assert_eq!(120.0, target.width());
        assert!(!clock.ticks.has_observers());
    }

    #[test]
    fn opacity_projection_stays_in_legal_range_even_with_spatial_overshoot() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let transition = width_transition(MaterialMotionRole::FastSpatial);
        transition.set_property(Some(Visual::opacity_property().as_property()));
        let target = border(&clock, transition.into(), 20.0);
        target.set_opacity(0.0);
        for i in 0..120 {
            clock.tick(i as f64 / 120.0);
            assert!((0.0..=1.0).contains(&target.opacity()));
        }
        assert_eq!(0.0, target.opacity());
        assert!(!clock.ticks.has_observers());
    }

    #[test]
    fn non_material_easing_uses_the_existing_duration_transition_contract() {
        let _app = crate::tests::support::start_themed_application();
        let clock = TestClock::new();
        let transition = width_transition(MaterialMotionRole::FastSpatial);
        transition.set_duration(TimeSpan::from_seconds(1.0));
        transition.set_easing(ferroui_base::animation::easings::LinearEasing::new());
        let target = border(&clock, transition.into(), 20.0);
        target.set_width(120.0);
        clock.tick(0.0);
        clock.tick(0.5);
        close(70.0, target.width());
        clock.tick(1.0);
        assert_eq!(120.0, target.width());
        target.disable_transitions();
        clock.tick(1.1); // The standard Clock releases its parent on the next tick.
        assert!(!clock.ticks.has_observers());
    }

    #[test]
    fn actual_button_template_uses_effects_spring_and_keeps_velocity_on_release() {
        crate::register_types();
        let clock = TestClock::new();
        let _app = UnitTestApplication::start(
            TestServices::styled_window()
                .with_global_clock(clock.clone())
                .with_theme(|| crate::MaterialTheme::new().as_style()),
        );
        let button = Button::new();
        button.set_content(Some(Rc::new("Press".to_string())));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&button)));
        window.show();
        window.update_layout();
        let surface = button
            .get_visual_descendants()
            .find(|node| node.name().as_deref() == Some("MaterialContainer"))
            .unwrap()
            .cast::<Border>()
            .unwrap();
        let spec = MaterialMotionScheme::Expressive.spring(MaterialMotionRole::DefaultEffects);
        button.pseudo_classes().set(":pressed", true);
        clock.tick(0.0);
        clock.tick(0.02);
        let incoming = spec.sample(0.02, 20.0, 8.0, 0.0);
        close(incoming.value, surface.corner_radius().top_left);
        button.pseudo_classes().set(":pressed", false);
        clock.tick(0.03);
        close(
            spec.sample(0.01, incoming.value, 20.0, incoming.velocity)
                .value,
            surface.corner_radius().top_left,
        );
        window.close();
    }
}
