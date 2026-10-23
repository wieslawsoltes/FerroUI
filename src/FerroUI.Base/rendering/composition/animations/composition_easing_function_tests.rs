//! Tests of this port for the WinUI easing functions
//! (`CompositionEasingFunction` and its subclasses).

use super::*;
use crate::animation::easings::{Easing, IEasing, SplineEasing};
use crate::numerics::Vector2;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::generated::ServerCompositionVisualProps;
use crate::rendering::composition::test_compositor::TestCompositor;
use std::f64::consts::PI;
use std::time::Duration;

use CompositionEasingFunctionMode::{In, InOut, Out};

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "expected {expected}, got {actual}");
}

/// The value of `easing` at `0`, `0.5` and `1`.
fn ends_and_middle(easing: &dyn IEasing) -> [f64; 3] {
    [easing.ease(0.0), easing.ease(0.5), easing.ease(1.0)]
}

#[test]
fn modes_mirror_the_ease_in_curve() {
    let power_in = PowerEasingFunction::new(In, 2.0);
    let power_out = PowerEasingFunction::new(Out, 2.0);
    let power_in_out = PowerEasingFunction::new(InOut, 2.0);
    assert_close(power_in.ease(0.5), 0.25);
    assert_close(power_out.ease(0.5), 0.75);
    assert_close(power_in_out.ease(0.25), 0.125);
    assert_close(power_in_out.ease(0.5), 0.5);
    assert_close(power_in_out.ease(0.75), 0.875);
    for easing in [power_in, power_out, power_in_out] {
        let [start, _, end] = ends_and_middle(&easing);
        assert_close(start, 0.0);
        assert_close(end, 1.0);
    }
    assert_eq!(power_out.mode(), Out);
    assert_eq!(power_out.power(), 2.0);
    // A negative power is taken as 0.
    assert_close(PowerEasingFunction::new(In, -1.0).ease(0.5), 1.0);
}

#[test]
fn back_easing_function() {
    let back = BackEasingFunction::new(In, 1.0);
    assert_eq!(back.amplitude(), 1.0);
    assert_eq!(back.mode(), In);
    assert_close(back.ease(0.0), 0.0);
    assert_close(back.ease(0.5), 0.125 - 0.5);
    assert_close(back.ease(1.0), 1.0);
    assert_close(BackEasingFunction::new(Out, 1.0).ease(0.5), 1.375);
    assert_close(BackEasingFunction::new(InOut, 1.0).ease(0.5), 0.5);
    // Without amplitude it is the cubic curve.
    assert_close(BackEasingFunction::new(In, 0.0).ease(0.5), 0.125);
}

#[test]
fn bounce_easing_function() {
    let bounce = BounceEasingFunction::new(Out, 3, 2.0);
    assert_eq!(bounce.bounces(), 3);
    assert_eq!(bounce.bounciness(), 2.0);
    let [start, _, end] = ends_and_middle(&bounce);
    assert_close(start, 0.0);
    assert_close(end, 1.0);
    let bounce_in = BounceEasingFunction::new(In, 3, 2.0);
    assert_close(bounce_in.ease(0.0), 0.0);
    assert_close(bounce_in.ease(1.0), 1.0);
    // The curve stays between 0 and 1 and touches 0 between the bounces.
    for i in 0..=100 {
        let value = bounce_in.ease(i as f64 / 100.0);
        assert!((-1e-9..=1.0 + 1e-9).contains(&value), "{value}");
    }
    // The first bounce ends at 1/11 of the time (bounces 3, bounciness 2:
    // 1 + 2 + 4 + 8/2 = 11 units).
    assert_close(bounce_in.ease(1.0 / 11.0), 0.0);
    // A bounciness of 1 or less is taken as just over 1.
    let flat = BounceEasingFunction::new(In, 2, 0.5);
    assert!(flat.ease(0.5).is_finite());
}

#[test]
fn circle_easing_function() {
    let circle = CircleEasingFunction::new(In);
    assert_close(circle.ease(0.0), 0.0);
    assert_close(circle.ease(0.5), 1.0 - 0.75f64.sqrt());
    assert_close(circle.ease(1.0), 1.0);
    assert_close(CircleEasingFunction::new(Out).ease(0.5), 0.75f64.sqrt());
}

#[test]
fn elastic_easing_function() {
    let elastic = ElasticEasingFunction::new(In, 3, 3.0);
    assert_eq!(elastic.oscillations(), 3);
    assert_eq!(elastic.springiness(), 3.0);
    assert_close(elastic.ease(0.0), 0.0);
    assert_close(elastic.ease(1.0), 1.0);
    let t: f64 = 0.5;
    let expo = ((3.0 * t).exp() - 1.0) / (3.0f64.exp() - 1.0);
    assert_close(elastic.ease(t), expo * ((PI * 6.0 + PI * 0.5) * t).sin());
    // Without springiness the envelope is linear.
    let linear_envelope = ElasticEasingFunction::new(In, 1, 0.0);
    assert_close(linear_envelope.ease(0.25), 0.25 * ((PI * 2.5) * 0.25).sin());
}

#[test]
fn exponential_easing_function() {
    let exponential = ExponentialEasingFunction::new(In, 2.0);
    assert_eq!(exponential.exponent(), 2.0);
    assert_close(exponential.ease(0.0), 0.0);
    assert_close(exponential.ease(0.5), (1f64.exp() - 1.0) / (2f64.exp() - 1.0));
    assert_close(exponential.ease(1.0), 1.0);
    // An exponent of 0 is linear.
    assert_close(ExponentialEasingFunction::new(In, 0.0).ease(0.3), 0.3);
}

#[test]
fn sine_easing_function() {
    let sine = SineEasingFunction::new(In);
    assert_close(sine.ease(0.0), 0.0);
    assert_close(sine.ease(0.5), 1.0 - (PI / 4.0).sin());
    assert_close(sine.ease(1.0), 1.0);
    assert_close(SineEasingFunction::new(Out).ease(0.5), (PI / 4.0).sin());
}

#[test]
fn linear_and_cubic_bezier_easing_functions() {
    let linear = LinearEasingFunction::new();
    assert_close(linear.ease(0.3), 0.3);

    let straight = CubicBezierEasingFunction::new(Vector2::new(0.0, 0.0), Vector2::new(1.0, 1.0));
    assert_close(straight.ease(0.5), 0.5);

    let curve = CubicBezierEasingFunction::new(Vector2::new(0.8, 0.0), Vector2::new(0.2, 1.0));
    assert_eq!(curve.control_point1(), Vector2::new(0.8, 0.0));
    assert_eq!(curve.control_point2(), Vector2::new(0.2, 1.0));
    let spline = SplineEasing::with_points(0.8f32 as f64, 0.0, 0.2f32 as f64, 1.0);
    // The key spline solver is iterative and starts from its previous solution, so two solvers
    // that evaluated different sequences agree to its tolerance only, which the steep middle of this curve magnifies.
    for t in [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
        assert!((curve.ease(t) - spline.ease(t)).abs() < 1e-3, "at {t}");
        assert!((curve.to_shared().ease(t) - spline.ease(t)).abs() < 1e-3, "at {t}");
    }

    let result = std::panic::catch_unwind(|| CubicBezierEasingFunction::new(Vector2::new(1.5, 0.0), Vector2::ONE));
    assert!(result.is_err());
}

#[test]
fn step_easing_function_defaults() {
    let step = StepEasingFunction::new();
    assert_eq!(step.step_count(), 1);
    assert_eq!(step.initial_step(), 0);
    assert_eq!(step.final_step(), 1);
    assert!(!step.is_initial_step_single_frame());
    assert!(!step.is_final_step_single_frame());
    // Steps 0 and 1 share the duration.
    assert_close(step.ease(0.0), 0.0);
    assert_close(step.ease(0.25), 0.0);
    assert_close(step.ease(0.75), 1.0);
    assert_close(step.ease(1.0), 1.0);

    // The final step follows the step count until it is set.
    step.set_step_count(5);
    assert_eq!(step.final_step(), 5);
    step.set_final_step(3);
    step.set_step_count(6);
    assert_eq!(step.final_step(), 3);
}

#[test]
fn step_easing_function_steps() {
    let step = StepEasingFunction::with_step_count(4);
    // Steps 0 to 4, a fifth of the duration each.
    assert_close(step.ease(0.1), 0.0);
    assert_close(step.ease(0.3), 0.25);
    assert_close(step.ease(0.5), 0.5);
    assert_close(step.ease(0.99), 1.0);

    // A single-frame final step: steps 0 to 3 share the duration, 4 is
    // reached at the end.
    step.set_is_final_step_single_frame(true);
    assert_close(step.ease(0.3), 0.25);
    assert_close(step.ease(0.99), 0.75);
    assert_close(step.ease(1.0), 1.0);

    // A single-frame initial step too: 0 at the start only.
    step.set_is_initial_step_single_frame(true);
    assert_close(step.ease(0.0), 0.0);
    assert_close(step.ease(0.1), 0.25);
    assert_close(step.ease(0.5), 0.5);
    assert_close(step.ease(0.9), 0.75);
    assert_close(step.ease(1.0), 1.0);

    // From step 1 to step 3.
    let partial = StepEasingFunction::with_step_count(4);
    partial.set_initial_step(1);
    partial.set_final_step(3);
    assert_close(partial.ease(0.0), 0.25);
    assert_close(partial.ease(0.5), 0.5);
    assert_close(partial.ease(1.0), 0.75);
}

#[test]
fn step_easing_function_shares_its_members_at_the_time() {
    let step = StepEasingFunction::with_step_count(2);
    let shared = step.to_shared();
    step.set_step_count(4);
    assert_close(shared.ease(0.7), 1.0);
    assert_close(step.ease(0.7), 0.75);
}

#[test]
fn parametric_easing_functions_are_shared_as_they_are() {
    let back = BackEasingFunction::new(Out, 0.5);
    assert_close(back.to_shared().ease(0.3), back.ease(0.3));
    let elastic = ElasticEasingFunction::new(InOut, 2, 4.0);
    assert_close(elastic.to_shared().ease(0.3), elastic.ease(0.3));
}

#[test]
fn factories_of_composition_easing_function_and_compositor() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let back = CompositionEasingFunction::create_back_easing_function(compositor, In, 1.0);
    assert_close(back.ease(0.5), -0.375);
    let cubic = CompositionEasingFunction::create_cubic_bezier_easing_function(
        compositor,
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 1.0),
    );
    assert_close(cubic.ease(0.4), 0.4);
    assert_close(CompositionEasingFunction::create_linear_easing_function(compositor).ease(0.4), 0.4);
    assert_close(CompositionEasingFunction::create_step_easing_function(compositor).ease(0.25), 0.0);
    assert_close(
        CompositionEasingFunction::create_step_easing_function_with_step_count(compositor, 4).ease(0.3),
        0.25,
    );
    assert_close(CompositionEasingFunction::create_bounce_easing_function(compositor, Out, 3, 2.0).ease(1.0), 1.0);
    assert_close(CompositionEasingFunction::create_circle_easing_function(compositor, In).ease(1.0), 1.0);
    assert_close(CompositionEasingFunction::create_elastic_easing_function(compositor, In, 3, 3.0).ease(0.0), 0.0);
    assert_close(CompositionEasingFunction::create_exponential_easing_function(compositor, In, 0.0).ease(0.2), 0.2);
    assert_close(CompositionEasingFunction::create_power_easing_function(compositor, In, 3.0).ease(0.5), 0.125);
    assert_close(CompositionEasingFunction::create_sine_easing_function(compositor, Out).ease(1.0), 1.0);

    let step = compositor.create_step_easing_function_with_step_count(4);
    assert_eq!(step.step_count(), 4);
    assert_eq!(compositor.create_step_easing_function().step_count(), 1);
    let bezier = compositor.create_cubic_bezier_easing_function(Vector2::new(0.25, 0.1), Vector2::new(0.25, 1.0));
    assert_eq!(bezier.control_point1(), Vector2::new(0.25, 0.1));

    // The key frames take them as they are or as an `Easing`.
    let animation = compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame_with_easing(0.5, 0.5, compositor.create_linear_easing_function());
    let easing: Easing = bezier.into();
    animation.insert_expression_key_frame(1.0, "1", Some(easing));
    assert_eq!(animation.key_frame_count(), 2);
}

#[test]
fn key_frame_animation_with_a_step_easing_function() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let target = compositor.create_solid_color_visual();
    fixture.run_jobs();

    let animation = compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, 1.0, compositor.create_step_easing_function_with_step_count(4));
    animation.set_duration(Duration::from_secs(1));
    let instance = animation.create_instance(target.server(), None).create();
    instance.resolve(compositor.server());
    instance.initialize(
        Duration::ZERO,
        ExpressionVariant::create(0f32),
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
    );
    let at = |t: f32| instance.evaluate(Duration::from_secs_f32(t), ExpressionVariant::create(0f32));
    assert_eq!(at(0.1), ExpressionVariant::Double(0.0));
    assert_eq!(at(0.3), ExpressionVariant::Double(0.25));
    assert_eq!(at(0.5), ExpressionVariant::Double(0.5));
}
