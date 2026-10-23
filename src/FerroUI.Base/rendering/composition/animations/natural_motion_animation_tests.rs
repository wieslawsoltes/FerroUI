//! Tests of this port for the spring natural motion animations
//! (`SpringScalarNaturalMotionAnimation`, `SpringVector2NaturalMotionAnimation`,
//! `SpringVector3NaturalMotionAnimation`).

use super::*;
use crate::numerics::{Vector2, Vector3};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{ServerCompositionPropertySet, ServerCompositionVisual};
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::{
    AsCompositionObject, CompositionBatchCompletedEventArgs, CompositionBatchTypes, CompositionPropertySet,
    CompositionSolidColorVisual, ICompositionObjectAnimations,
};
use crate::threading::Dispatcher;
use crate::Vector3D;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// A compositor with one visual whose server side is active.
///
/// The fields drop in order: the visual before the compositor of the test.
struct Fixture {
    server: Rc<ServerCompositionVisual>,
    visual: CompositionSolidColorVisual,
    test: TestCompositor,
}

impl Fixture {
    fn new() -> Fixture {
        let test = TestCompositor::new();
        let visual = test.compositor.create_solid_color_visual();
        visual.set_opacity(0.0);
        test.run_jobs();
        let server = test.server::<ServerCompositionVisual>(visual.server());
        server.activate();
        Fixture { server, visual, test }
    }

    /// Renders frames for `duration` of the server clock, then runs the jobs
    /// of the dispatcher (where `Completed` is raised).
    fn run_for(&self, duration: Duration) {
        let server = self.test.compositor.server();
        let started = server.server_now();
        while server.server_now() < started + duration {
            self.test.render_loop.tick();
        }
        Dispatcher::ui_thread().run_jobs(None);
    }

    /// Renders frames for `duration` and returns the smallest and the largest
    /// value an entry of a property set took.
    fn sample_for(&self, set: &CompositionPropertySet, name: &str, duration: Duration) -> (f32, f32) {
        let server = self.test.compositor.server();
        let started = server.server_now();
        let (mut min, mut max) = (f32::MAX, f32::MIN);
        while server.server_now() < started + duration {
            self.test.render_loop.tick();
            let value = entry(&self.test, set, name).cast_or_default::<f32>();
            min = min.min(value);
            max = max.max(value);
        }
        (min, max)
    }
}

/// The current value of an entry of a live property set on the server.
fn entry(test: &TestCompositor, set: &CompositionPropertySet, name: &str) -> ExpressionVariant {
    let id = set.as_composition_object().server().expect("a live property set");
    test.server::<ServerCompositionPropertySet>(id).get_value(name)
}

/// A counter of `Completed`.
fn counter() -> (Rc<Cell<u32>>, impl Fn(&CompositionBatchCompletedEventArgs) + 'static) {
    let count = Rc::new(Cell::new(0));
    let handler = {
        let count = count.clone();
        move |_: &CompositionBatchCompletedEventArgs| count.set(count.get() + 1)
    };
    (count, handler)
}

/// The largest displacement a spring from rest at `x0 = 1` reaches on the
/// other side of the rest point within one second.
fn overshoot(zeta: f64) -> f64 {
    let omega = std::f64::consts::TAU / 0.05;
    (0..=10_000)
        .map(|i| spring_state(1.0, 0.0, omega, zeta, i as f64 * 1e-4).0)
        .fold(0.0_f64, |most, x| most.max(-x))
}

#[test]
fn spring_state_starts_at_the_initial_state() {
    for zeta in [0.0, 0.3, 1.0, 2.5] {
        let (x, v) = spring_state(2.0, -3.0, 10.0, zeta, 0.0);
        assert_eq!((x, v), (2.0, -3.0));
        // The velocity is the derivative of the position.
        let h = 1e-6;
        let (x1, v1) = spring_state(2.0, -3.0, 10.0, zeta, 0.1);
        let (x2, _) = spring_state(2.0, -3.0, 10.0, zeta, 0.1 + h);
        assert!(((x2 - x1) / h - v1).abs() < 1e-3, "zeta {zeta}: {} vs {v1}", (x2 - x1) / h);
    }
}

#[test]
fn spring_state_overshoots_only_below_critical_damping() {
    // The overshoot of an under-damped spring: exp(-pi zeta / sqrt(1 - zeta^2)).
    let expected = (-std::f64::consts::PI * 0.3 / (1.0f64 - 0.09).sqrt()).exp();
    assert!((overshoot(0.3) - expected).abs() < 1e-3, "{} vs {expected}", overshoot(0.3));
    assert!(overshoot(0.9) > 0.0);
    assert_eq!(overshoot(1.0), 0.0);
    assert_eq!(overshoot(2.0), 0.0);
    // Undamped: it swings to the other side as far as it started.
    assert!((overshoot(0.0) - 1.0).abs() < 1e-3);
}

#[test]
fn spring_state_comes_to_rest() {
    let omega = std::f64::consts::TAU / 0.05;
    for zeta in [0.3, 1.0, 2.0] {
        let (x, v) = spring_state(100.0, 50.0, omega, zeta, 2.0);
        assert!(x.abs() < 1e-6 && v.abs() < 1e-4, "zeta {zeta}: {x} {v}");
    }
}

#[test]
fn spring_defaults() {
    let fixture = Fixture::new();
    let spring = fixture.test.compositor.create_spring_scalar_animation();
    assert_eq!(spring.damping_ratio(), 1.0);
    assert_eq!(spring.period(), Duration::from_millis(50));
    assert_eq!(spring.final_value(), None);
    assert_eq!(spring.initial_value(), None);
    assert_eq!(spring.initial_velocity(), 0.0);
    assert_eq!(spring.delay_time(), Duration::ZERO);
    assert_eq!(spring.delay_behavior(), AnimationDelayBehavior::SetInitialValueAfterDelay);
    assert_eq!(spring.stop_behavior(), AnimationStopBehavior::LeaveCurrentValue);
    let vector = fixture.test.compositor.create_spring_vector2_animation();
    assert_eq!(vector.initial_velocity(), Vector2::new(0.0, 0.0));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| spring.set_period(Duration::ZERO))).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| spring.set_damping_ratio(-1.0))).is_err());
}

#[test]
fn scalar_spring_settles_at_the_final_value_and_completes_its_batch() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let spring = compositor.create_spring_scalar_animation();
    spring.set_final_value(Some(1.0));
    fixture.visual.start_animation("Opacity", &*spring);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    assert!(fixture.server.opacity() < 1.0, "the spring is moving");
    assert_eq!(completed.get(), 0);

    fixture.run_for(Duration::from_millis(600));
    assert_eq!(fixture.server.opacity(), 1.0);
    assert_eq!(completed.get(), 1);
}

#[test]
fn vector3_spring_animates_the_offset_of_a_visual() {
    let fixture = Fixture::new();
    let spring = fixture.test.compositor.create_spring_vector3_animation();
    spring.set_final_value(Some(Vector3::new(100.0, 50.0, 0.0)));
    spring.set_damping_ratio(0.5);
    fixture.visual.start_animation("Offset", &*spring);
    fixture.test.run_jobs();
    assert!(fixture.server.offset().x < 100.0);
    fixture.run_for(Duration::from_millis(800));
    assert_eq!(fixture.server.offset(), Vector3D::new(100.0, 50.0, 0.0));
}

#[test]
fn vector2_spring_starts_from_its_initial_value() {
    let fixture = Fixture::new();
    let set = fixture.test.compositor.create_property_set();
    set.insert_vector2("P", Vector2::new(5.0, 5.0));
    let spring = fixture.test.compositor.create_spring_vector2_animation();
    spring.set_initial_value(Some(Vector2::new(-10.0, 20.0)));
    spring.set_final_value(Some(Vector2::new(0.0, 0.0)));
    set.start_animation("P", &*spring);
    fixture.test.run_jobs();
    let first = entry(&fixture.test, &set, "P").cast_or_default::<Vector2>();
    assert!(first.x < -5.0 && first.y > 10.0, "{first:?}");
    fixture.run_for(Duration::from_millis(600));
    assert_eq!(entry(&fixture.test, &set, "P").cast_or_default::<Vector2>(), Vector2::new(0.0, 0.0));
}

#[test]
fn under_damped_spring_overshoots() {
    let fixture = Fixture::new();
    let set = fixture.test.compositor.create_property_set();
    set.insert_scalar("X", 0.0);
    let spring = fixture.test.compositor.create_spring_scalar_animation();
    spring.set_final_value(Some(1.0));
    spring.set_damping_ratio(0.2);
    set.start_animation("X", &*spring);
    fixture.test.run_jobs();
    let (_, max) = fixture.sample_for(&set, "X", Duration::from_millis(1000));
    assert!(max > 1.2, "the spring overshoots: {max}");
    fixture.run_for(Duration::from_millis(200));
    assert_eq!(entry(&fixture.test, &set, "X"), ExpressionVariant::Double(1.0));
}

#[test]
fn critically_and_over_damped_springs_do_not_overshoot() {
    // One compositor per test: the fixture is shared by the damping ratios.
    let fixture = Fixture::new();
    for damping in [1.0, 2.0] {
        let set = fixture.test.compositor.create_property_set();
        set.insert_scalar("X", 0.0);
        let spring = fixture.test.compositor.create_spring_scalar_animation();
        spring.set_final_value(Some(1.0));
        spring.set_damping_ratio(damping);
        set.start_animation("X", &*spring);
        fixture.test.run_jobs();
        let (min, max) = fixture.sample_for(&set, "X", Duration::from_millis(1000));
        assert!(max <= 1.0, "damping {damping}: {max}");
        assert!(min >= 0.0, "damping {damping}: {min}");
        assert_eq!(entry(&fixture.test, &set, "X"), ExpressionVariant::Double(1.0), "damping {damping}");
    }
}

#[test]
fn implicit_spring_moves_to_the_new_value() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let spring = compositor.create_spring_vector3_animation();
    spring.set_target(Some("Offset".to_owned()));
    let implicit = compositor.create_implicit_animation_collection();
    implicit.insert("Offset", spring.clone());
    fixture.visual.set_implicit_animations(Some(implicit));

    fixture.visual.set_offset(Vector3D::new(30.0, 0.0, 0.0));
    fixture.test.run_jobs();
    assert!(fixture.server.offset().x < 30.0, "the spring is moving");
    fixture.run_for(Duration::from_millis(600));
    assert_eq!(fixture.server.offset(), Vector3D::new(30.0, 0.0, 0.0));
}

#[test]
fn undamped_spring_is_an_infinite_animation() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let finite = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let all = compositor.create_scoped_batch(CompositionBatchTypes::ALL_ANIMATIONS);
    let spring = compositor.create_spring_scalar_animation();
    spring.set_final_value(Some(1.0));
    spring.set_damping_ratio(0.0);
    fixture.visual.start_animation("Opacity", &*spring);
    finite.end();
    all.end();
    let (finite_completed, finite_handler) = counter();
    let _finite = finite.completed(finite_handler);
    let (all_completed, all_handler) = counter();
    let _all = all.completed(all_handler);

    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(200));
    assert_eq!(finite_completed.get(), 1);
    assert_eq!(all_completed.get(), 0, "the spring oscillates forever");

    fixture.visual.stop_animation("Opacity");
    fixture.test.run_jobs();
    assert_eq!(all_completed.get(), 1);
}

#[test]
fn stopped_spring_applies_its_stop_behavior() {
    let fixture = Fixture::new();
    let spring = fixture.test.compositor.create_spring_scalar_animation();
    spring.set_final_value(Some(0.75));
    spring.set_period(Duration::from_secs(30));
    spring.set_stop_behavior(AnimationStopBehavior::SetToFinalValue);
    fixture.visual.start_animation("Opacity", &*spring);
    fixture.test.run_jobs();
    assert!(fixture.server.opacity() < 0.75);
    fixture.visual.stop_animation("Opacity");
    fixture.test.run_jobs();
    assert_eq!(fixture.server.opacity(), 0.75);
}
