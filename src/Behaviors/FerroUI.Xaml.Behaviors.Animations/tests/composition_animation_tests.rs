//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/CompositionAnimationTests.cs`.
//!
//! Adapted: the composition visuals are those of a window of [`CompositionApp`]; the offsets of
//! composition visuals are double-precision vectors, compared with the single-precision vectors
//! of the reference converted. The `try`/`finally` blocks that close the window of a test are
//! straight-line code: a failed assertion ends the test (and its application) anyway.

use super::support::{control, start_app, CompositionApp};
use crate::composition::composition_animation_helpers::CompositionAnimationHelpers;
use crate::{
    AttentionAnimations, EntranceAnimations, ExitAnimations, FramerMotionAnimations, OrbitAnimation, ParallaxAnimation,
    SlidingAnimation, TiltAnimation,
};
use ferroui_base::numerics::{Quaternion, Vector3};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::{Point, Rect, Ref, Size, Vector, Vector3D};
use ferroui_controls::{Border, Canvas, Control};
use std::time::Duration;

/// The composition offset of `target`, if it has a composition visual.
fn composition_offset(target: &Ref<Border>) -> Option<Vector3D> {
    ElementComposition::get_element_visual(target).map(|visual| visual.offset())
}

/// A 100 by 100 border at (30, 40) of a 300 by 300 canvas.
fn positioned_border() -> (Ref<Border>, Ref<Canvas>) {
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    Canvas::set_left(&target, 30.0);
    Canvas::set_top(&target, 40.0);
    let canvas = Canvas::new();
    canvas.set_width(300.0);
    canvas.set_height(300.0);
    canvas.children().add(target.clone());
    (target, canvas)
}

#[test]
fn parallax_animation_calculates_proportional_offset() {
    let _app = start_app();
    let offset = ParallaxAnimation::calculate_offset(Vector::new(20.0, 50.0), 0.25);

    assert_eq!(offset, Vector3::new(5.0, 12.5, 0.0));
}

#[test]
fn parallax_animation_applies_offset_to_attached_composition_visual() {
    let app = CompositionApp::start();
    let (target, canvas) = positioned_border();
    let window = app.window(&canvas.clone().upcast());
    window.show();
    app.run_jobs();

    let applied = ParallaxAnimation::apply_to(Some(control(&target)), Vector::new(20.0, 50.0), 0.25);

    assert!(applied);
    assert_eq!(composition_offset(&target), Some(Vector3::new(35.0, 52.5, 0.0).into()));
    window.close();
}

#[test]
fn parallax_animation_retains_attached_composition_visual_across_updates() {
    let app = CompositionApp::start();
    let (target, canvas) = positioned_border();
    let window = app.window(&canvas.clone().upcast());
    window.show();
    app.run_jobs();
    let animation = ParallaxAnimation::try_create(Some(control(&target)));

    let animation = animation.expect("the target has a composition visual");
    animation.apply(Vector::new(10.0, 20.0), 0.5);
    animation.apply(Vector::new(20.0, 50.0), 0.25);

    assert_eq!(composition_offset(&target), Some(Vector3::new(35.0, 52.5, 0.0).into()));
    window.close();
}

#[test]
fn orbit_animation_calculates_normalized_orientation() {
    let _app = start_app();
    let orientation = OrbitAnimation::calculate_orientation(Quaternion::IDENTITY, Vector::new(10.0, 5.0), 0.5);

    assert_ne!(orientation, Quaternion::IDENTITY);
    assert!((0.9999..=1.0001).contains(&orientation.length()));
}

#[test]
fn tilt_animation_returns_identity_at_center_and_for_empty_size() {
    let _app = start_app();
    let center = TiltAnimation::calculate_orientation(Size::new(100.0, 80.0), Point::new(50.0, 40.0), 5.0);
    let empty = TiltAnimation::calculate_orientation(Size::default(), Point::new(10.0, 10.0), 5.0);

    assert_eq!(center, Quaternion::IDENTITY);
    assert_eq!(empty, Quaternion::IDENTITY);
}

#[test]
fn tilt_animation_calculates_orientation_away_from_center() {
    let _app = start_app();
    let orientation = TiltAnimation::calculate_orientation(Size::new(100.0, 100.0), Point::new(100.0, 50.0), 5.0);

    assert_ne!(orientation, Quaternion::IDENTITY);
}

#[test]
fn tilt_animation_does_not_calculate_orientation_at_center_or_for_empty_size() {
    let _app = start_app();
    let center = TiltAnimation::try_calculate_orientation(Size::new(100.0, 80.0), Point::new(50.0, 40.0), 5.0);
    let empty = TiltAnimation::try_calculate_orientation(Size::default(), Point::new(10.0, 10.0), 5.0);

    assert!(center.is_none());
    assert!(empty.is_none());
    // Adapted: the reference also checks that its `out` parameter is the identity; here there is
    // no orientation when none is calculated, and the orientation used then is the identity.
    assert_eq!(center.unwrap_or(Quaternion::IDENTITY), Quaternion::IDENTITY);
    assert_eq!(empty.unwrap_or(Quaternion::IDENTITY), Quaternion::IDENTITY);
}

#[test]
fn tilt_animation_animates_back_to_neutral_at_center() {
    let app = CompositionApp::start();
    let target = Border::new();
    target.set_width(100.0);
    target.set_height(80.0);
    let window = app.window(&target.clone().upcast());
    window.show();
    app.run_jobs();

    assert!(TiltAnimation::apply(Some(control(&target)), Point::new(100.0, 40.0), 5.0));
    assert!(TiltAnimation::apply(Some(control(&target)), Point::new(50.0, 40.0), 5.0));
    window.close();
}

#[test]
fn composition_effects_return_false_without_target_visuals() {
    let _app = start_app();
    let orbit = OrbitAnimation::new();

    assert!(ParallaxAnimation::try_create(None).is_none());
    assert!(!ParallaxAnimation::apply_to(None, Vector::default(), 0.25));
    assert!(!orbit.rotate(None, Vector::new(10.0, 5.0), 0.5));
    assert!(!TiltAnimation::apply(None, Point::default(), 5.0));
    assert!(!TiltAnimation::reset(None));
    assert_eq!(orbit.orientation(), Quaternion::IDENTITY);
}

#[test]
fn attention_animation_starts_from_positioned_control_layout_offset() {
    assert_composition_offset(|target| AttentionAnimations::set_bounce(target, 10_000.0), Vector3::new(30.0, 40.0, 0.0));
}

#[test]
fn entrance_animation_starts_from_layout_relative_offset() {
    assert_composition_offset(
        |target| EntranceAnimations::set_slide_in_left(target, 10_000.0),
        Vector3::new(-210.0, 40.0, 0.0),
    );
}

#[test]
fn exit_animation_starts_from_positioned_control_layout_offset() {
    assert_composition_offset(
        |target| ExitAnimations::set_slide_out_down(target, 10_000.0),
        Vector3::new(30.0, 40.0, 0.0),
    );
}

#[test]
fn framer_motion_animation_starts_from_layout_relative_offset() {
    assert_composition_offset(
        |target| FramerMotionAnimations::set_slide_in_from_left(target, 10_000.0),
        Vector3::new(-90.0, 40.0, 0.0),
    );
}

#[test]
fn sliding_animation_does_not_overwrite_positioned_control_layout_offset() {
    assert_composition_offset(|target| SlidingAnimation::set_left(target, 10_000.0), Vector3::new(30.0, 40.0, 0.0));
}

#[test]
fn composition_animation_helpers_add_relative_motion_to_layout_offset() {
    let _app = start_app();
    let target = Border::new();
    target.arrange(Rect::new(30.0, 40.0, 100.0, 100.0));

    let offset = CompositionAnimationHelpers::get_layout_offset_with(control(&target), Vector3::new(-100.0, 25.0, 0.0));

    assert_eq!(offset, Vector3::new(-70.0, 65.0, 0.0));
}

/// `AssertCompositionOffsetAsync`: configures a positioned border, shows it and checks the
/// offset of its composition visual once the animation has started.
fn assert_composition_offset(configure: impl Fn(&Control), expected_offset: Vector3) {
    let app = CompositionApp::start();
    let (target, canvas) = positioned_border();
    let window = app.window(&canvas.clone().upcast());
    configure(control(&target));

    window.show();
    app.run_jobs();
    // `await Task.Delay(20)`.
    std::thread::sleep(Duration::from_millis(20));
    app.run_jobs();

    assert_eq!(composition_offset(&target), Some(expected_offset.into()));
    window.close();
}
