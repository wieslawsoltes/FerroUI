//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/CompositionCatalogSmokeTests.cs`.
//!
//! Adapted: the windows are those of [`CompositionApp`]; the offsets and scales of composition
//! visuals are double-precision vectors, compared with the single-precision vectors of the
//! reference converted. The `try`/`finally` blocks that close the window of a test are
//! straight-line code: a failed assertion ends the test (and its application) anyway.

use super::support::{control, CompositionApp};
use crate::composition::composition_animation_helpers::CompositionAnimationHelpers;
use crate::{
    AttentionAnimations, EntranceAnimations, ExitAnimations, FadeAnimation, FramerMotionAnimations, RotateAnimation,
    ScaleAnimation, SlidingAnimation, SpecialAnimations,
};
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{Ref, Vector3D, Visual};
use ferroui_controls::{Border, Canvas, Control, StackPanel};
use std::rc::Rc;

const DURATION_MILLISECONDS: f64 = 10_000.0;

#[test]
fn attention_catalog_starts_every_animation() {
    assert_catalog_starts(&[
        |target| AttentionAnimations::set_bounce(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_flash(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_pulse(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_rubber_band(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_shake_x(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_shake_y(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_head_shake(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_swing(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_tada(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_wobble(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_jello(target, DURATION_MILLISECONDS),
        |target| AttentionAnimations::set_heart_beat(target, DURATION_MILLISECONDS),
    ]);
}

#[test]
fn entrance_catalog_starts_every_animation() {
    assert_catalog_starts(&[
        |target| EntranceAnimations::set_back_in_down(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_back_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_back_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_back_in_up(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_bounce_in(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_bounce_in_down(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_bounce_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_bounce_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_bounce_in_up(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_down(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_down_big(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_left_big(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_right_big(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_up(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_up_big(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_top_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_top_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_bottom_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_bottom_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_fade_in_zoom(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_flip_in_x(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_flip_in_y(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_light_speed_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_light_speed_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_rotate_in(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_rotate_in_down_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_rotate_in_down_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_rotate_in_up_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_rotate_in_up_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_slide_in_down(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_slide_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_slide_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_slide_in_up(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_zoom_in(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_zoom_in_down(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_zoom_in_left(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_zoom_in_right(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_zoom_in_up(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_jack_in_the_box(target, DURATION_MILLISECONDS),
        |target| EntranceAnimations::set_roll_in(target, DURATION_MILLISECONDS),
    ]);
}

#[test]
fn exit_catalog_starts_every_animation() {
    assert_catalog_starts(&[
        |target| ExitAnimations::set_back_out_down(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_back_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_back_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_back_out_up(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_bounce_out(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_bounce_out_down(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_bounce_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_bounce_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_bounce_out_up(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_down(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_down_big(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_left_big(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_right_big(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_up(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_up_big(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_top_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_top_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_bottom_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_fade_out_bottom_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_flip_out_x(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_flip_out_y(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_light_speed_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_light_speed_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_rotate_out(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_rotate_out_down_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_rotate_out_down_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_rotate_out_up_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_rotate_out_up_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_slide_out_down(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_slide_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_slide_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_slide_out_up(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_zoom_out(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_zoom_out_down(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_zoom_out_left(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_zoom_out_right(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_zoom_out_up(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_hinge(target, DURATION_MILLISECONDS),
        |target| ExitAnimations::set_roll_out(target, DURATION_MILLISECONDS),
    ]);
}

#[test]
fn framer_motion_catalog_starts_every_animation() {
    assert_catalog_starts(&[
        |target| FramerMotionAnimations::set_fade_in(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_fade_in_up(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_fade_in_down(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_fade_in_left(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_fade_in_right(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_slide_in_from_left(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_slide_in_from_right(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_slide_in_from_top(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_slide_in_from_bottom(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_pop_in(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_pop_out(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_spring_in(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_spring_out(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_rotate_in(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_rotate_out(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_scale_in(target, DURATION_MILLISECONDS),
        |target| FramerMotionAnimations::set_scale_out(target, DURATION_MILLISECONDS),
    ]);
}

#[test]
fn primitive_catalogs_start_every_animation() {
    assert_catalog_starts(&[
        |target| FadeAnimation::set_fade_in(target, DURATION_MILLISECONDS),
        |target| FadeAnimation::set_fade_out(target, DURATION_MILLISECONDS),
        |target| FadeAnimation::set_custom_fade(target, 0.2, 0.8, DURATION_MILLISECONDS),
        |target| SlidingAnimation::set_left(target, DURATION_MILLISECONDS),
        |target| SlidingAnimation::set_right(target, DURATION_MILLISECONDS),
        |target| SlidingAnimation::set_top(target, DURATION_MILLISECONDS),
        |target| SlidingAnimation::set_bottom(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_scale_in(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_scale_out(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_zoom_in(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_zoom_out(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_bounce(target, DURATION_MILLISECONDS),
        |target| ScaleAnimation::set_custom_scale(target, 0.8, 0.9, 1.0, 1.0, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_rotate_clockwise(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_rotate_counter_clockwise(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_rotate_in(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_rotate_out(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_flip(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_swing(target, DURATION_MILLISECONDS),
        |target| RotateAnimation::set_custom_rotate(target, -30.0, 30.0, DURATION_MILLISECONDS),
        |target| SpecialAnimations::set_flip(target, DURATION_MILLISECONDS),
    ]);
}

#[test]
fn zero_duration_applies_final_composition_states_without_starting_invalid_animations() {
    let app = CompositionApp::start();
    let new_target = || {
        let target = Border::new();
        target.set_width(32.0);
        target.set_height(32.0);
        target
    };
    let attention = new_target();
    let entrance = new_target();
    let exit = new_target();
    let fade = new_target();
    let scale = new_target();
    let rotate = new_target();
    let sliding = new_target();
    let special = new_target();
    let framer_motion = new_target();
    let targets = [&attention, &entrance, &exit, &fade, &scale, &rotate, &sliding, &special, &framer_motion];
    let canvas = Canvas::new();
    canvas.set_width(300.0);
    canvas.set_height(400.0);
    for target in targets {
        Canvas::set_left(target, 30.0);
        Canvas::set_top(target, 40.0);
        canvas.children().add(target.clone());
    }

    AttentionAnimations::set_bounce(control(&attention), 0.0);
    EntranceAnimations::set_slide_in_left(control(&entrance), 0.0);
    ExitAnimations::set_slide_out_down(control(&exit), 0.0);
    FadeAnimation::set_custom_fade(control(&fade), 0.2, 0.8, 0.0);
    ScaleAnimation::set_scale_out(control(&scale), 0.0);
    RotateAnimation::set_custom_rotate(control(&rotate), -30.0, 30.0, 0.0);
    SlidingAnimation::set_left(control(&sliding), 0.0);
    SpecialAnimations::set_flip(control(&special), 0.0);
    FramerMotionAnimations::set_slide_in_from_left(control(&framer_motion), 0.0);

    let window = app.window(&canvas.clone().upcast());
    window.show();
    app.run_jobs();

    let offset = |x: f32, y: f32| -> Vector3D { Vector3::new(x, y, 0.0).into() };
    assert_eq!(get_visual(&attention).offset(), offset(30.0, 40.0));
    assert_eq!(get_visual(&entrance).offset(), offset(30.0, 40.0));
    assert_eq!(get_visual(&exit).offset(), offset(30.0, 280.0));
    assert_eq!(get_visual(&fade).opacity(), 0.8);
    assert_eq!(get_visual(&scale).scale(), Vector3D::from(Vector3::ZERO));
    assert_eq!(get_visual(&rotate).rotation_angle(), CompositionAnimationHelpers::degrees_to_radians(30.0));
    assert_eq!(get_visual(&sliding).offset(), offset(30.0, 40.0));
    assert_eq!(get_visual(&special).scale(), Vector3D::from(Vector3::ONE));
    assert_eq!(get_visual(&special).rotation_angle(), 0.0);
    assert_eq!(get_visual(&framer_motion).offset(), offset(30.0, 40.0));
    window.close();
}

/// `AssertCatalogStarts`: configures one border per animation, shows them and checks that each
/// has a composition visual.
fn assert_catalog_starts(configure_animations: &[fn(&Control)]) {
    let app = CompositionApp::start();
    let panel = StackPanel::new();
    for configure in configure_animations {
        let target = Border::new();
        target.set_width(32.0);
        target.set_height(32.0);
        configure(control(&target));
        panel.children().add(target);
    }

    let window = app.window(&panel.clone().upcast());
    window.show();
    app.run_jobs();

    for target in panel.children().snapshot().iter() {
        assert!(ElementComposition::get_element_visual(target).is_some());
    }
    window.close();
}

/// `GetVisual`: the composition visual of `target`, which it must have.
fn get_visual(target: &Ref<Border>) -> Rc<CompositionVisual> {
    let visual: &Visual = target;
    ElementComposition::get_element_visual(visual).expect("the target has a composition visual")
}
