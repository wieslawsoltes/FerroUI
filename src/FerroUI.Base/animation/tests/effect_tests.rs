use super::*;
use crate::animation::transitions::EffectTransition;
use crate::animation::{Animation, Cue, KeyFrame, Transitions};
use crate::media::effects::{BlurEffect, DropShadowEffect, IEffect, ImmutableBlurEffect, ImmutableDropShadowEffect};
use crate::media::Colors;
use crate::styling::Setter;
use crate::threading::CancellationToken;

type EffectValue = Option<Rc<dyn IEffect>>;

fn blur(radius: f64) -> EffectValue {
    Some(Rc::new(ImmutableBlurEffect::new(radius)))
}

fn effect_animation(from: EffectValue, to: EffectValue) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(Visual::effect_property(), from) as _]));
    animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Visual::effect_property(), to) as _]));
    animation
}

#[test]
fn blur_effect_key_frames_are_interpolated() {
    let mutable = BlurEffect::new();
    mutable.set_radius(10.0);
    let animation = effect_animation(blur(0.0), Some(mutable.into()));
    let border = Border::new();
    let clock = TestClock::new();
    let run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    let effect = border.effect().expect("an effect");
    assert_eq!(effect.as_blur_effect().expect("a blur").radius(), 5.0);

    clock.step(seconds(1.0));
    assert!(run.is_completed());
    assert!(border.effect().is_none());
}

#[test]
fn drop_shadow_effect_key_frames_are_interpolated() {
    let from: EffectValue = Some(Rc::new(ImmutableDropShadowEffect::new(0.0, 0.0, 0.0, Colors::BLACK, 0.0)));
    let to = DropShadowEffect::new();
    to.set_offset_x(10.0);
    to.set_offset_y(20.0);
    to.set_blur_radius(4.0);
    to.set_color(Colors::BLACK);
    to.set_opacity(1.0);
    let animation = effect_animation(from, Some(to.into()));
    let border = Border::new();
    let clock = TestClock::new();
    let _run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    let effect = border.effect().expect("an effect");
    let shadow = effect.as_drop_shadow_effect().expect("a drop shadow");
    assert_eq!(shadow.offset_x(), 5.0);
    assert_eq!(shadow.offset_y(), 10.0);
    assert_eq!(shadow.blur_radius(), 2.0);
    assert_eq!(shadow.opacity(), 0.5);
}

#[test]
fn key_frames_of_different_effect_kinds_switch_at_half() {
    let shadow: EffectValue = Some(Rc::new(ImmutableDropShadowEffect::new(1.0, 1.0, 1.0, Colors::BLACK, 1.0)));
    let animation = effect_animation(blur(4.0), shadow);
    let border = Border::new();
    let clock = TestClock::new();
    let _run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.25));
    assert!(border.effect().unwrap().as_blur_effect().is_some());
    clock.step(seconds(0.75));
    assert!(border.effect().unwrap().as_drop_shadow_effect().is_some());
}

#[test]
fn effect_transition_fades_a_blur_in_from_no_effect() {
    start();
    let clock = TestClock::new();
    let transition = EffectTransition::new();
    transition.set_property(Some(Visual::effect_property()));
    transition.set_duration(seconds(1.0));
    let border = Border::new();
    border.set_transitions(Some(Transitions::from_items([transition.into()])));
    let root = TestRoot::with_child(&border);
    root.set_clock(Some(clock.as_clock()));

    border.set_effect(blur(8.0));

    clock.step(seconds(0.0));
    assert_eq!(border.effect().unwrap().as_blur_effect().unwrap().radius(), 0.0);
    clock.step(seconds(0.5));
    assert_eq!(border.effect().unwrap().as_blur_effect().unwrap().radius(), 4.0);
    clock.step(seconds(1.0));
    assert_eq!(border.effect().unwrap().as_blur_effect().unwrap().radius(), 8.0);
    assert!(!border.is_animating(Visual::effect_property()));

    // And out again: the missing effect stands for a blur of radius zero.
    border.set_effect(None);
    clock.step(seconds(1.0));
    clock.step(seconds(1.5));
    assert_eq!(border.effect().unwrap().as_blur_effect().unwrap().radius(), 4.0);
    clock.step(seconds(2.0));
    assert!(border.effect().is_none());
}

#[test]
fn effect_transition_from_a_blur_to_a_drop_shadow_fades_the_shadow_in() {
    start();
    let clock = TestClock::new();
    let transition = EffectTransition::new();
    transition.set_property(Some(Visual::effect_property()));
    transition.set_duration(seconds(1.0));
    let border = Border::new();
    border.set_effect(blur(8.0));
    border.set_transitions(Some(Transitions::from_items([transition.into()])));
    let root = TestRoot::with_child(&border);
    root.set_clock(Some(clock.as_clock()));

    border.set_effect(Some(Rc::new(ImmutableDropShadowEffect::new(10.0, 0.0, 4.0, Colors::BLACK, 1.0))));

    // The old value is not a drop shadow, so the shadow transitions from a
    // drop shadow without a visible result.
    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    let effect = border.effect().unwrap();
    let shadow = effect.as_drop_shadow_effect().expect("a drop shadow");
    assert_eq!(shadow.blur_radius(), 2.0);
    assert_eq!(shadow.opacity(), 0.5);
    assert_close(shadow.offset_x(), 5.0, 1e-9);
}
