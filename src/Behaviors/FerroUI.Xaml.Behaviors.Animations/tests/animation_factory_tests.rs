//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/AnimationFactoryTests.cs`.
//!
//! Adapted: `Assert.Throws<ArgumentOutOfRangeException>` checks for the panic that stands for
//! the exception; the theories run every inline data row in one test.

use super::support::{control, panics, run_jobs, start_app};
use crate::{AnimationFactory, FluidMoveAnimation};
use ferroui_base::animation::{Cue, FillMode, TimeSpan};
use ferroui_base::media::{ITransform, RotateTransform, TranslateTransform};
use ferroui_controls::Border;
use std::rc::Rc;

#[test]
fn create_fade_in_creates_delayed_opacity_animation() {
    let _app = start_app();
    let delay = TimeSpan::from_milliseconds(200.0);
    let duration = TimeSpan::from_milliseconds(300.0);

    let animation = AnimationFactory::create_fade_in(delay, duration);

    assert_eq!(animation.duration(), TimeSpan::from_milliseconds(500.0));
    let key_times: Vec<TimeSpan> = animation.children().snapshot().iter().map(|key_frame| key_frame.key_time()).collect();
    assert_eq!(key_times, vec![TimeSpan::ZERO, delay, delay + duration]);
}

#[test]
fn create_fade_in_rejects_negative_times() {
    let _app = start_app();
    for (delay_milliseconds, duration_milliseconds) in [(-1.0, 0.0), (0.0, -1.0)] {
        assert!(panics(|| {
            AnimationFactory::create_fade_in(
                TimeSpan::from_milliseconds(delay_milliseconds),
                TimeSpan::from_milliseconds(duration_milliseconds),
            );
        }));
    }
}

#[test]
fn create_fade_in_timeline_preserves_explicit_key_times() {
    let _app = start_app();
    let delay = TimeSpan::from_milliseconds(500.0);
    let completion = TimeSpan::from_milliseconds(650.0);
    let total_duration = TimeSpan::from_milliseconds(750.0);

    let animation = AnimationFactory::create_fade_in_timeline(delay, completion, total_duration);

    assert_eq!(animation.duration(), total_duration);
    let key_times: Vec<TimeSpan> = animation.children().snapshot().iter().map(|key_frame| key_frame.key_time()).collect();
    assert_eq!(key_times, vec![TimeSpan::ZERO, delay, completion]);
}

#[test]
fn create_fade_in_timeline_rejects_non_chronological_key_times() {
    let _app = start_app();
    for (delay_milliseconds, completion_milliseconds, duration_milliseconds) in
        [(500.0, 250.0, 750.0), (500.0, 800.0, 750.0)]
    {
        assert!(panics(|| {
            AnimationFactory::create_fade_in_timeline(
                TimeSpan::from_milliseconds(delay_milliseconds),
                TimeSpan::from_milliseconds(completion_milliseconds),
                TimeSpan::from_milliseconds(duration_milliseconds),
            );
        }));
    }
}

#[test]
fn fluid_move_animation_creates_translation_animation() {
    let _app = start_app();
    let duration = TimeSpan::from_milliseconds(250.0);

    let animation = FluidMoveAnimation::create(12.0, -8.0, duration);

    assert_eq!(animation.duration(), duration);
    assert_eq!(animation.fill_mode(), FillMode::Forward);
    let key_frames = animation.children().snapshot();
    assert_eq!(key_frames.len(), 2);
    assert_eq!(key_frames[0].cue(), Cue::new(0.0));
    assert_eq!(key_frames[0].setters().count(), 2);
    assert_eq!(key_frames[1].cue(), Cue::new(1.0));
    assert_eq!(key_frames[1].setters().count(), 2);
}

#[test]
fn fluid_move_animation_rejects_negative_duration() {
    let _app = start_app();
    assert!(panics(|| {
        FluidMoveAnimation::create(0.0, 0.0, TimeSpan::from_milliseconds(-1.0));
    }));
}

#[test]
fn fluid_move_animation_try_run_prepares_control_transform() {
    let app = start_app();
    let target = Border::new();
    let rotate: Rc<dyn ITransform> = RotateTransform::new().into();
    target.set_render_transform(Some(rotate));

    let started = FluidMoveAnimation::try_run(Some(control(&target)), 12.0, -8.0, TimeSpan::ZERO);

    assert!(started);
    let transform = target
        .render_transform()
        .and_then(|transform| {
            transform.as_object().and_then(|object| object.downcast_ref::<TranslateTransform>()).map(|t| t.to_ref())
        })
        .expect("the render transform is a translate transform");
    run_jobs();
    // Adapted: the headless platform of the reference ticks the global clock; here the test
    // ticks it.
    app.clock.pulse(TimeSpan::ZERO);
    app.clock.pulse(TimeSpan::from_milliseconds(1.0));
    run_jobs();
    assert_eq!(transform.x(), 0.0);
    assert_eq!(transform.y(), 0.0);
    assert!(!FluidMoveAnimation::try_run(None, 0.0, 0.0, TimeSpan::ZERO));
}
