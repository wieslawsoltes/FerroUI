use super::*;
use crate::animation::transitions::EffectTransition;
use crate::animation::{Animation, Cue, KeyFrame, Transitions};
use crate::media::effects::{
    BlurEffect, CustomEffect, DropShadowEffect, EffectParameterValue, IEffect, ImmutableBlurEffect, ImmutableCustomEffect,
    ImmutableDropShadowEffect,
};
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

// --- custom effects (#455, not from upstream) ------------------------------------

/// A custom effect whose neutral value is an amount of zero.
#[derive(Clone, Debug, Default, PartialEq)]
struct Fade {
    amount: f64,
}

impl CustomEffect for Fade {
    const NAME: &'static str = "animation-test-fade";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("amount", self.amount.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { amount: 0.0 })
    }
}

/// A custom effect without a neutral value.
#[derive(Clone, Debug, Default, PartialEq)]
struct Stepped {
    amount: f64,
}

impl CustomEffect for Stepped {
    const NAME: &'static str = "animation-test-stepped";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("amount", self.amount.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
            _ => false,
        }
    }
}

fn fade(amount: f64) -> EffectValue {
    Some(ImmutableCustomEffect::new(Fade { amount }).into())
}

fn stepped(amount: f64) -> EffectValue {
    Some(ImmutableCustomEffect::new(Stepped { amount }).into())
}

/// The amount of a fade effect.
fn fade_amount(effect: EffectValue) -> Option<f64> {
    effect.and_then(|effect| {
        effect.as_custom_effect().and_then(|custom| custom.downcast_ref::<Fade>()).map(|fade| fade.amount)
    })
}

fn custom_transition_border(clock: &Rc<TestClock>) -> (Ref<Border>, Ref<TestRoot>) {
    let transition = EffectTransition::new();
    transition.set_property(Some(Visual::effect_property()));
    transition.set_duration(seconds(1.0));
    let border = Border::new();
    border.set_transitions(Some(Transitions::from_items([transition.into()])));
    let root = TestRoot::with_child(&border);
    root.set_clock(Some(clock.as_clock()));
    (border, root)
}

#[test]
fn custom_effect_key_frames_are_interpolated() {
    let animation = effect_animation(fade(0.0), fade(10.0));
    let border = Border::new();
    let clock = TestClock::new();
    let _run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    assert_eq!(Some(5.0), fade_amount(border.effect()));
}

#[test]
fn key_frames_of_custom_effects_of_different_types_switch_at_half() {
    let animation = effect_animation(fade(4.0), stepped(8.0));
    let border = Border::new();
    let clock = TestClock::new();
    let _run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.25));
    assert_eq!(Some(4.0), fade_amount(border.effect()));
    clock.step(seconds(0.75));
    let effect = border.effect().expect("an effect");
    assert!(effect.as_custom_effect().is_some_and(|custom| custom.is::<Stepped>()));
}

#[test]
fn effect_transition_fades_a_custom_effect_in_from_its_neutral_value() {
    start();
    let clock = TestClock::new();
    let (border, _root) = custom_transition_border(&clock);

    border.set_effect(fade(8.0));
    clock.step(seconds(0.0));
    assert_eq!(Some(0.0), fade_amount(border.effect()));
    clock.step(seconds(0.5));
    assert_eq!(Some(4.0), fade_amount(border.effect()));
    clock.step(seconds(1.0));
    assert_eq!(Some(8.0), fade_amount(border.effect()));

    // From one value to another.
    border.set_effect(fade(4.0));
    clock.step(seconds(1.0));
    clock.step(seconds(1.5));
    assert_eq!(Some(6.0), fade_amount(border.effect()));
    clock.step(seconds(2.0));

    // And out again, to the neutral value and then no effect.
    border.set_effect(None);
    clock.step(seconds(2.0));
    clock.step(seconds(2.5));
    assert_eq!(Some(2.0), fade_amount(border.effect()));
    clock.step(seconds(3.0));
    assert!(border.effect().is_none());
}

#[test]
fn effect_transition_switches_a_custom_effect_without_a_neutral_value_at_half() {
    start();
    let clock = TestClock::new();
    let (border, _root) = custom_transition_border(&clock);

    border.set_effect(stepped(8.0));
    clock.step(seconds(0.0));
    assert!(border.effect().is_none());
    clock.step(seconds(0.75));
    let effect = border.effect().expect("the new effect after half of the transition");
    let custom = effect.as_custom_effect().and_then(|custom| custom.downcast_ref::<Stepped>());
    assert_eq!(Some(&Stepped { amount: 8.0 }), custom);
}
