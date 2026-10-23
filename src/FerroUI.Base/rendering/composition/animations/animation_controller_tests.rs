//! Tests of this port for `AnimationController`, `Compositor.GlobalPlaybackRate`,
//! `CompositionAnimation.InitialValueExpressions`,
//! `ColorKeyFrameAnimation.InterpolationColorSpace` and
//! `CompositionObject.Properties` / `Comment`.

use super::*;
use crate::animation::easings::LinearEasing;
use crate::media::Color;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{ServerCompositionPropertySet, ServerCompositionVisual};
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::{
    AsCompositionObject, CompositionBatchCompletedEventArgs, CompositionBatchTypes, CompositionPropertySet,
    CompositionSolidColorVisual, Compositor, ICompositionObjectAnimations,
};
use crate::threading::Dispatcher;
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

    /// A live property set with the scalar entry `name`.
    fn set_with(&self, name: &str, value: f32) -> Rc<CompositionPropertySet> {
        let set = self.test.compositor.create_property_set();
        set.insert_scalar(name, value);
        set
    }

    /// A scalar animation to `value` over `duration`, with a linear easing.
    fn scalar_animation(&self, value: f32, duration: Duration) -> Rc<ScalarKeyFrameAnimation> {
        let animation = self.test.compositor.create_scalar_key_frame_animation();
        animation.insert_key_frame_with_easing(1.0, value, Rc::new(LinearEasing::new()));
        animation.set_duration(duration);
        animation
    }

    /// The current value of a scalar entry of a live property set on the
    /// server.
    fn scalar(&self, set: &CompositionPropertySet, name: &str) -> f32 {
        self.entry(set, name).cast_or_default::<f32>()
    }

    fn entry(&self, set: &CompositionPropertySet, name: &str) -> ExpressionVariant {
        let id = set.as_composition_object().server().expect("a live property set");
        self.test.server::<ServerCompositionPropertySet>(id).get_value(name)
    }
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

#[test]
fn controller_defaults_and_ranges() {
    let fixture = Fixture::new();
    let controller = fixture.test.compositor.create_animation_controller();
    assert_eq!(controller.progress(), 0.0);
    assert_eq!(controller.playback_rate(), 1.0);
    assert!(!controller.is_paused());
    assert_eq!(controller.progress_behavior(), AnimationControllerProgressBehavior::Default);
    controller.set_progress(2.0);
    assert_eq!(controller.progress(), 1.0);
    controller.set_playback_rate(5000.0);
    assert_eq!(controller.playback_rate(), AnimationController::MAX_PLAYBACK_RATE);
    controller.set_progress_behavior(AnimationControllerProgressBehavior::IncludesDelayTime);
    assert_eq!(controller.progress_behavior(), AnimationControllerProgressBehavior::IncludesDelayTime);
    controller.pause();
    assert!(controller.is_paused());
    controller.resume();
    assert!(!controller.is_paused());
}

#[test]
fn paused_controller_scrubs_a_key_frame_animation() {
    let fixture = Fixture::new();
    let set = fixture.set_with("A", 0.0);
    let controller = fixture.test.compositor.create_animation_controller();
    controller.pause();
    controller.set_progress(0.25);
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    set.start_animation_with_controller("A", &*animation, &controller);
    fixture.test.run_jobs();
    assert_eq!(fixture.scalar(&set, "A"), 0.25);

    controller.set_progress(0.75);
    fixture.test.run_jobs();
    assert_eq!(fixture.scalar(&set, "A"), 0.75);

    // Paused: the time of the animation stands still.
    fixture.run_for(Duration::from_millis(50));
    assert_eq!(fixture.scalar(&set, "A"), 0.75);

    // The same progress again moves the animation back to it.
    controller.resume();
    // The resume is committed before its time is measured.
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(50));
    assert!(fixture.scalar(&set, "A") > 0.75);
    controller.pause();
    controller.set_progress(0.75);
    fixture.test.run_jobs();
    assert_eq!(fixture.scalar(&set, "A"), 0.75);

    assert!(Rc::ptr_eq(&set.try_get_animation_controller("A").expect("a controller"), &controller));
}

#[test]
fn paused_animation_does_not_end_until_resumed() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let set = fixture.set_with("A", 0.0);
    let controller = compositor.create_animation_controller();
    controller.pause();
    controller.set_progress(1.0);
    let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.scalar_animation(1.0, Duration::from_millis(100));
    set.start_animation_with_controller("A", &*animation, &controller);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(200));
    assert_eq!(fixture.scalar(&set, "A"), 1.0);
    assert_eq!(completed.get(), 0, "a paused animation does not end");

    controller.resume();
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(50));
    assert_eq!(completed.get(), 1);
    assert_eq!(fixture.scalar(&set, "A"), 1.0);
}

#[test]
fn playback_rate_scales_and_reverses_the_time_of_the_animation() {
    let fixture = Fixture::new();
    let set = fixture.set_with("A", 0.0);
    let controller = fixture.test.compositor.create_animation_controller();
    controller.set_playback_rate(0.0);
    controller.set_progress(0.5);
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    set.start_animation_with_controller("A", &*animation, &controller);
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(50));
    assert_eq!(fixture.scalar(&set, "A"), 0.5, "a rate of 0 stops the time");

    controller.set_playback_rate(-1.0);
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(100));
    let value = fixture.scalar(&set, "A");
    assert!(value < 0.41, "a negative rate plays backwards: {value}");
    fixture.run_for(Duration::from_millis(500));
    assert_eq!(fixture.scalar(&set, "A"), 0.0, "the animation stops at its start");
}

#[test]
fn progress_behavior_includes_the_delay() {
    let fixture = Fixture::new();
    let set = fixture.set_with("A", 0.0);
    let controller = fixture.test.compositor.create_animation_controller();
    controller.pause();
    controller.set_progress_behavior(AnimationControllerProgressBehavior::IncludesDelayTime);
    controller.set_progress(0.75);
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    animation.set_delay_time(Duration::from_secs(1));
    set.start_animation_with_controller("A", &*animation, &controller);
    fixture.test.run_jobs();
    // 0.75 of the two seconds is half way through the animation.
    assert_eq!(fixture.scalar(&set, "A"), 0.5);
}

#[test]
fn controller_progress_can_be_animated_by_an_expression() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let set = fixture.set_with("A", 0.0);
    let source = fixture.set_with("P", 0.4);
    let controller = compositor.create_animation_controller();
    controller.pause();
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    set.start_animation_with_controller("A", &*animation, &controller);
    let scrub = compositor.create_expression_animation_with("source.P");
    scrub.set_reference_parameter("source", source.clone());
    controller.start_animation("Progress", &*scrub);
    fixture.test.run_jobs();
    fixture.test.run_jobs();
    assert!((fixture.scalar(&set, "A") - 0.4).abs() < 1e-6);

    source.insert_scalar("P", 0.6);
    fixture.test.run_jobs();
    fixture.test.run_jobs();
    assert!((fixture.scalar(&set, "A") - 0.6).abs() < 1e-6);
}

#[test]
fn try_get_animation_controller_takes_a_running_animation_over() {
    let fixture = Fixture::new();
    assert!(fixture.visual.try_get_animation_controller("Opacity").is_none(), "nothing runs");
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    fixture.visual.start_animation("Opacity", &*animation);
    fixture.test.run_jobs();

    let controller = fixture.visual.try_get_animation_controller("Opacity").expect("a controller");
    assert!(Rc::ptr_eq(&controller, &fixture.visual.try_get_animation_controller("Opacity").unwrap()));
    controller.pause();
    controller.set_progress(0.5);
    // The controller is attached after the frame's animations ran; the next
    // frame moves the animation.
    fixture.test.run_jobs();
    fixture.test.run_jobs();
    assert_eq!(fixture.server.opacity(), 0.5);

    fixture.visual.stop_animation("Opacity");
    assert!(fixture.visual.try_get_animation_controller("Opacity").is_none());
    fixture.test.run_jobs();
}

#[test]
fn controller_pauses_a_spring() {
    let fixture = Fixture::new();
    let set = fixture.set_with("X", 0.0);
    let controller = fixture.test.compositor.create_animation_controller();
    controller.pause();
    let spring = fixture.test.compositor.create_spring_scalar_animation();
    spring.set_final_value(Some(1.0));
    set.start_animation_with_controller("X", &*spring, &controller);
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(300));
    assert!(fixture.scalar(&set, "X") < 0.1, "the paused spring has not moved");
    controller.resume();
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(500));
    assert_eq!(fixture.scalar(&set, "X"), 1.0);
}

#[test]
fn global_playback_rate_scales_every_animation() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    assert_eq!(compositor.global_playback_rate(), 1.0);
    compositor.set_global_playback_rate(0.0);
    assert_eq!(compositor.global_playback_rate(), 0.0);
    fixture.test.run_jobs();

    let animation = fixture.scalar_animation(1.0, Duration::from_millis(100));
    fixture.visual.start_animation("Opacity", &*animation);
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(200));
    assert_eq!(fixture.server.opacity(), 0.0, "the time of the animations stands still");

    compositor.set_global_playback_rate(1.0);
    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(200));
    assert_eq!(fixture.server.opacity(), 1.0);

    compositor.set_global_playback_rate(-3.0);
    assert_eq!(compositor.global_playback_rate(), Compositor::MIN_GLOBAL_PLAYBACK_RATE);
    fixture.test.run_jobs();
}

#[test]
fn initial_value_expressions_set_properties_when_the_animation_starts() {
    let fixture = Fixture::new();
    fixture.visual.set_opacity(0.1);
    fixture.test.run_jobs();
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(30));
    animation.initial_value_expressions().insert("Opacity", "0.25");
    // Reads the animated property as it was before the animation.
    animation.initial_value_expressions().insert("RotationAngle", "this.Target.Opacity + 1");
    fixture.visual.start_animation("Opacity", &*animation);
    fixture.test.run_jobs();
    assert!((fixture.server.opacity() - 0.25).abs() < 0.01, "{}", fixture.server.opacity());
    assert!((fixture.server.rotation_angle() - 1.1).abs() < 1e-6, "{}", fixture.server.rotation_angle());
}

#[test]
fn initial_value_expressions_of_a_property_set_entry() {
    let fixture = Fixture::new();
    let set = fixture.set_with("A", 3.0);
    let controller = fixture.test.compositor.create_animation_controller();
    controller.pause();
    controller.set_progress(0.0);
    let animation = fixture.scalar_animation(10.0, Duration::from_secs(1));
    animation.initial_value_expressions().insert("A", "this.StartingValue * 2");
    animation.initial_value_expressions().insert("B", "7");
    set.start_animation_with_controller("A", &*animation, &controller);
    fixture.test.run_jobs();
    assert_eq!(fixture.scalar(&set, "A"), 6.0);
    assert_eq!(fixture.scalar(&set, "B"), 7.0);

    // An expression animation starts from the initial value too.
    let expression = fixture.test.compositor.create_expression_animation_with("this.StartingValue + 1");
    expression.initial_value_expressions().insert("A", "20");
    set.start_animation("A", &*expression);
    fixture.test.run_jobs();
    assert_eq!(fixture.scalar(&set, "A"), 21.0);
}

#[test]
fn initial_value_expression_that_cannot_be_parsed_fails_the_start() {
    let fixture = Fixture::new();
    let animation = fixture.scalar_animation(1.0, Duration::from_secs(1));
    animation.initial_value_expressions().insert("Opacity", "1 +");
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fixture.visual.start_animation("Opacity", &*animation)));
    assert!(result.is_err());
    fixture.test.run_jobs();
}

#[test]
fn color_animation_interpolates_in_its_color_space() {
    let red = Color::from_argb(255, 255, 0, 0);
    let blue = Color::from_argb(255, 0, 0, 255);
    let fixture = Fixture::new();
    for (space, expected) in [
        (CompositionColorSpace::Auto, Color::from_argb(255, 127, 0, 127)),
        (CompositionColorSpace::Rgb, Color::from_argb(255, 127, 0, 127)),
        (CompositionColorSpace::Hsl, Color::from_argb(255, 255, 0, 255)),
        (CompositionColorSpace::HslLinear, Color::from_argb(255, 255, 0, 255)),
        (CompositionColorSpace::RgbLinear, Color::from_argb(255, 188, 0, 188)),
    ] {
        // One compositor per test: the fixture is shared by the colour spaces.
        let compositor = &fixture.test.compositor;
        let set = compositor.create_property_set();
        set.insert_color("C", red);
        let controller = compositor.create_animation_controller();
        controller.pause();
        controller.set_progress(0.5);
        let animation = compositor.create_color_key_frame_animation();
        assert_eq!(animation.interpolation_color_space(), CompositionColorSpace::Auto);
        animation.set_interpolation_color_space(space);
        animation.insert_key_frame_with_easing(1.0, blue, Rc::new(LinearEasing::new()));
        animation.set_duration(Duration::from_secs(1));
        set.start_animation_with_controller("C", &*animation, &controller);
        fixture.test.run_jobs();
        assert_eq!(fixture.entry(&set, "C").cast_or_default::<Color>(), expected, "{space:?}");
    }
}

#[test]
fn every_object_has_a_property_set_and_a_comment() {
    let fixture = Fixture::new();
    let object = fixture.visual.composition_object();
    assert_eq!(object.comment(), "");
    object.set_comment("header".to_owned());
    assert_eq!(object.comment(), "header");

    let properties = object.properties();
    assert!(Rc::ptr_eq(&properties, &object.properties()));
    assert!(properties.is_live());
    properties.insert_scalar("Weight", 0.3);
    // An expression reads the entries through a reference to the set.
    let expression = fixture.test.compositor.create_expression_animation_with("props.Weight");
    expression.set_reference_parameter("props", properties.clone());
    fixture.visual.start_animation("Opacity", &*expression);
    fixture.test.run_jobs();
    assert!((fixture.server.opacity() - 0.3).abs() < 1e-6);
}
