use super::*;
use crate::animation::transitions::DoubleTransition;
use crate::animation::{
    Animatable, Animation, FillMode, IterationCount, KeyFrame, PlaybackDirection, Transitions,
};
use crate::data::BindingPriority;
use crate::media::Brushes;
use crate::controls::ResourceKey;
use crate::styling::test_support::try_attach;
use crate::styling::{ControlTheme, Selectors, Setter, Style};
use crate::threading::CancellationToken;

fn create_target() -> Rc<MockTransition> {
    MockTransition::new(Visual::opacity_property())
}

fn create_control(transition: &Rc<MockTransition>) -> (Ref<Border>, Ref<TestRoot>) {
    let control = Border::new();
    control.set_transitions(Some(Transitions::from_items([transition.handle()])));
    let root = TestRoot::with_child(&control);
    (control, root)
}

fn create_styled_control(
    transition1: Option<Rc<MockTransition>>,
    transition2: Option<Rc<MockTransition>>,
) -> (Ref<Border>, Ref<TestRoot>) {
    let transition1 = transition1.unwrap_or_else(create_target);
    let transition2 = transition2.unwrap_or_else(|| MockTransition::new(Layoutable::width_property()));

    let control = Border::new();
    control.styles().add(Style::with_setters(
        Selectors::of_type::<Border>(),
        [Setter::new(Animatable::transitions_property(), Some(Transitions::from_items([transition1.handle()])))],
    ));
    control.styles().add(Style::with_setters(
        Selectors::of_type::<Border>().class("foo"),
        [Setter::new(Animatable::transitions_property(), Some(Transitions::from_items([transition2.handle()])))],
    ));

    let root = TestRoot::with_child(&control);
    (control, root)
}

/// A root with a clock and a style that applies `animation` to borders,
/// and a border in it.
fn animated_border(animation: &Ref<Animation>) -> (Rc<TestClock>, Ref<TestRoot>, Ref<Border>) {
    let clock = TestClock::new();
    let root = TestRoot::new();
    root.set_clock(Some(clock.as_clock()));
    let style = Style::with_selector(Selectors::of_type::<Border>());
    style.add_animation(animation);
    root.styles().add(style);
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    root.set_child(&target);
    (clock, root, target)
}

fn opacity_animation(from: f64, to: f64, duration: f64) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(duration));
    animation.children().add(KeyFrame::with_key_time(seconds(0.0), [Setter::new(Visual::opacity_property(), from) as _]));
    animation.children().add(KeyFrame::with_key_time(seconds(1.0), [Setter::new(Visual::opacity_property(), to) as _]));
    animation
}

#[test]
fn transition_is_not_applied_when_not_attached_to_visual_tree() {
    let target = create_target();
    let control = Border::new();
    control.set_transitions(Some(Transitions::from_items([target.handle()])));

    control.set_opacity(0.5);

    assert_eq!(target.call_count(), 0);
}

#[test]
fn transition_is_not_applied_to_initial_style() {
    start();
    let target = create_target();
    let control = Border::new();
    control.set_transitions(Some(Transitions::from_items([target.handle()])));

    let root = TestRoot::new();
    root.styles()
        .add(Style::with_setters(Selectors::of_type::<Border>(), [Setter::new(Visual::opacity_property(), 0.8)]));

    root.set_child(&control);

    assert_eq!(control.opacity(), 0.8);
    assert_eq!(target.call_count(), 0);
}

#[test]
fn transition_is_applied_when_local_value_changes() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_opacity(0.5);

    assert!(target.was_applied(1.0, 0.5));
}

#[test]
fn transition_is_not_applied_when_animated_value_changes() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_value_with_priority(Visual::opacity_property(), 0.5, BindingPriority::Animation);

    assert_eq!(target.call_count(), 0);
}

#[test]
fn invalid_values_in_animation_should_not_crash_animations() {
    start();
    // A missing value and a value of the wrong type.
    let invalid_values: [Option<BoxedValue>; 2] = [None, Some(Rc::new("stringValue".to_string()))];
    for invalid_value in invalid_values {
        let keyframe1 = KeyFrame::with_key_time(seconds(0.0), []);
        let setter = Setter::empty();
        setter.set_property(Some(Layoutable::width_property()));
        setter.set_value(Some(crate::styling::SetterValue::Value(Rc::new(1.0_f64))));
        keyframe1.setters().add(setter);

        let keyframe2 = KeyFrame::with_key_time(seconds(2.0), []);
        let setter = Setter::empty();
        setter.set_property(Some(Layoutable::width_property()));
        setter.set_value(invalid_value.map(crate::styling::SetterValue::Value));
        keyframe2.setters().add(setter);

        let animation = Animation::new();
        animation.set_duration(seconds(2.0));
        animation.children().add(keyframe1);
        animation.children().add(keyframe2);
        animation.set_iteration_count(IterationCount::INFINITE);
        animation.set_playback_direction(PlaybackDirection::Alternate);

        let (clock, _root, _target) = animated_border(&animation);

        clock.step(seconds(0.0));
        clock.step(seconds(1.0));
        clock.step(seconds(2.0));
        clock.step(seconds(3.0));
    }
}

#[test]
fn transition_is_not_applied_when_style_trigger_changes_with_local_value_present() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_value(Visual::opacity_property(), 0.5);

    assert!(target.was_applied(1.0, 0.5));
    target.clear();

    control.set_value_with_priority(Visual::opacity_property(), 0.8, BindingPriority::StyleTrigger);

    assert_eq!(target.call_count(), 0);
}

#[test]
fn transition_is_disposed_when_local_value_changes() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_opacity(0.5);
    target.disposed.set(0);
    control.set_opacity(0.4);

    assert_eq!(target.disposed.get(), 1);
}

#[test]
fn new_transition_is_applied_when_local_value_changes() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    let first = Cell::new(true);
    *target.on_apply.borrow_mut() = Some(Rc::new(move |control: &Animatable| {
        if first.replace(false) {
            control.set_value_with_priority(Visual::opacity_property(), 0.9, BindingPriority::Animation);
        }
    }));

    control.set_opacity(0.5);

    assert_eq!(control.opacity(), 0.9);
    target.clear();

    control.set_opacity(0.4);

    assert!(target.was_applied(0.9, 0.4));
}

#[test]
fn transition_is_not_applied_when_removed_from_visual_tree() {
    start();
    let target = create_target();
    let (control, root) = create_control(&target);

    control.set_opacity(0.5);

    assert!(target.was_applied(1.0, 0.5));
    target.clear();

    root.remove_child(&control);
    control.set_opacity(0.8);

    assert_eq!(target.call_count(), 0);
}

#[test]
fn animation_is_cancelled_when_transition_removed() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_opacity(0.5);
    control.transitions().unwrap().remove_at(0);

    assert_eq!(target.disposed.get(), 1);
}

#[test]
fn animation_is_cancelled_when_new_style_activates() {
    start();
    let target = create_target();
    let (control, _root) = create_styled_control(Some(target.clone()), None);

    control.set_opacity(0.5);

    assert_eq!(target.calls.borrow().as_slice(), &[ApplyCall { old_value: 1.0, new_value: 0.5 }]);

    control.classes().add("foo");

    assert_eq!(target.disposed.get(), 1);
}

#[test]
fn transition_from_style_trigger_is_applied() {
    start();
    let target = MockTransition::new(Layoutable::width_property());
    let (control, _root) = create_styled_control(None, Some(target.clone()));

    control.classes().add("foo");
    control.set_width(100.0);

    let calls = target.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].old_value.is_nan());
    assert_eq!(calls[0].new_value, 100.0);
}

fn opacity_transitions() -> Transitions {
    let transition = DoubleTransition::new();
    transition.set_property(Some(Visual::opacity_property()));
    transition.set_duration(seconds(1.0));
    Transitions::from_items([transition.into()])
}

#[test]
fn replacing_transitions_during_animation_does_not_throw_key_not_found() {
    start();
    let clock = TestClock::new();
    let root = TestRoot::new();
    root.set_clock(Some(clock.as_clock()));
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Border>(),
        [Setter::new(Animatable::transitions_property(), Some(opacity_transitions()))],
    ));
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Border>().class("foo"),
        [
            Setter::new(Animatable::transitions_property(), Some(opacity_transitions())),
            Setter::new(Visual::opacity_property(), 0.0),
        ],
    ));
    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    root.set_child(&target);

    target.classes().add("foo");
    clock.step(seconds(0.0));
    clock.step(seconds(0.5));

    assert_eq!(target.opacity(), 0.5);

    target.classes().remove("foo");
}

#[test]
fn transitions_can_be_changed_to_collection_that_contains_the_same_transitions() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    control.set_transitions(Some(Transitions::from_items([target.handle()])));
}

#[test]
fn run_normal_use_case_animation() {
    start();
    let animation = opacity_animation(1.0, 0.5, 10.0);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    clock.step(seconds(0.99));

    assert!((0.5..=0.51).contains(&target.opacity()), "{}", target.opacity());
}

#[test]
fn run_normal_use_case_animation_with_infinite_iteration() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_iteration_count(IterationCount::INFINITE);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));

    for (time, expected) in [(0.5, 0.5), (1.0, 0.0), (1.5, 0.5), (2.0, 0.0)] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "at {time}");
    }
}

#[test]
fn changing_speed_ratio_keeps_playback_time_constant() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));

    clock.step(seconds(0.25));
    animation.set_speed_ratio(2.0);
    assert_eq!(target.opacity(), 0.25);
    clock.step(seconds(0.25));
    assert_eq!(target.opacity(), 0.25);

    clock.step(seconds(0.25 + 0.125));
    assert_eq!(target.opacity(), 0.5);
    animation.set_speed_ratio(0.5);
    assert_eq!(target.opacity(), 0.5);
    clock.step(seconds(0.25 + 0.125));
    assert_eq!(target.opacity(), 0.5);

    clock.step(seconds(0.25 + 0.125 + 0.5));
    assert_eq!(target.opacity(), 0.75);
    animation.set_speed_ratio(1.0);
    assert_eq!(target.opacity(), 0.75);
    clock.step(seconds(0.25 + 0.125 + 0.5));
    assert_eq!(target.opacity(), 0.75);

    clock.step(seconds(0.25 + 0.125 + 0.5 + 0.25));
    assert_eq!(target.opacity(), 1.0);
}

#[test]
fn reversing_direction_past_initial_point_clamps_to_initial_point() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_delay(seconds(0.5));
    animation.set_playback_direction(PlaybackDirection::Normal);
    let (clock, _root, target) = animated_border(&animation);

    for (time, expected) in [(0.0, 1.0), (0.5, 1.0), (1.0, 0.5)] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "at {time}");
    }

    animation.set_playback_direction(PlaybackDirection::Reverse);
    for (time, expected) in [(1.0, 0.5), (1.2, 0.3), (1.5, 0.0), (2.0, 0.0), (3.0, 0.0)] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "reversed at {time}");
    }

    animation.set_playback_direction(PlaybackDirection::Normal);
    for (time, expected) in [(3.0, 0.0), (3.5, 0.0), (4.0, 0.5), (4.5, 1.0)] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "resumed at {time}");
    }
}

#[test]
fn changing_speed_ratio_every_frame_should_not_lose_playback_time() {
    start();
    // One second of playback out of a ten second animation.
    let animation = {
        let animation = Animation::new();
        animation.set_duration(seconds(10.0));
        animation.children().add(KeyFrame::with_key_time(seconds(0.0), [Setter::new(Visual::opacity_property(), 0.0) as _]));
        animation.children().add(KeyFrame::with_key_time(seconds(10.0), [Setter::new(Visual::opacity_property(), 1.0) as _]));
        animation
    };
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));

    // 60 frames of 1/60th of a second. SpeedRatio stays effectively 1 but is
    // written on every frame, as it would be when bound to a slider being
    // dragged. Re-anchoring playback time on each change must not
    // accumulate rounding errors.
    let frame = TimeSpan::from_ticks(TimeSpan::TICKS_PER_SECOND / 60);
    let mut time = TimeSpan::ZERO;

    for i in 0..60 {
        time += frame;
        animation.set_speed_ratio(1.0 + i as f64 * 1e-9);
        clock.step(time);
    }

    assert_close(target.opacity(), 0.1, 0.001);
}

#[test]
fn reversing_infinite_animation_past_initial_point_replays_previous_iterations() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_delay(seconds(0.5));
    animation.set_iteration_count(IterationCount::INFINITE);
    animation.set_fill_mode(FillMode::Both);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    assert_eq!(target.opacity(), 0.0);

    // Initial delay, then half of the first iteration.
    clock.step(seconds(1.0));
    assert_eq!(target.opacity(), 0.5);

    animation.set_playback_direction(PlaybackDirection::Reverse);
    for (time, expected) in [
        (1.0, 0.5),
        (1.25, 0.25),
        // Back at the initial point.
        (1.5, 0.0),
        // The initial delay is mirrored around the initial point, so
        // crossing it while reversed takes twice the delay.
        (2.0, 0.0),
        (2.5, 0.0),
        // Past the delay, the iteration before the initial one plays
        // backwards.
        (2.75, 0.75),
        (3.0, 0.5),
        (3.5, 1.0),
    ] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "at {time}");
    }
}

#[test]
fn clock_is_inherited() {
    let clock = TestClock::new();
    let root = TestRoot::new();
    let child = Border::new();
    root.set_child(&child);
    assert!(child.clock().is_none());
    root.set_clock(Some(clock.as_clock()));
    assert!(child.clock() == Some(clock.as_clock()));
}

#[test]
fn transitions_collection_changes_are_tracked_while_attached() {
    start();
    let (control, _root) = {
        let control = Border::new();
        control.set_transitions(Some(Transitions::new()));
        let root = TestRoot::with_child(&control);
        (control, root)
    };
    let target = create_target();

    control.set_opacity(0.9);
    assert_eq!(target.call_count(), 0);

    control.transitions().unwrap().add(target.handle());
    control.set_opacity(0.5);
    assert!(target.was_applied(0.9, 0.5));

    control.transitions().unwrap().clear();
    assert_eq!(target.disposed.get(), 1);
    target.clear();
    control.set_opacity(0.2);
    assert_eq!(target.call_count(), 0);
}

#[test]
fn transitions_can_re_set_during_styling() {
    start();
    let target = create_target();
    let (control, _root) = create_control(&target);

    // Assigning and then clearing Transitions ensures we have a transition state
    // collection created.
    control.clear_value(Animatable::transitions_property());

    control.values().begin_styling();

    // Setting opacity then Transitions means that we receive the Transitions change
    // after the Opacity change when EndStyling is called.
    let style = Style::new();
    style.add_setter(Setter::new(Visual::opacity_property(), 0.5));
    style.add_setter(Setter::new(
        Animatable::transitions_property(),
        Some(Transitions::from_items([target.handle()])),
    ));

    try_attach(&style, &control, None);

    // Which means that the transition state hasn't been initialized with the new
    // Transitions when the Opacity change notification gets raised here.
    control.values().end_styling(&control);
}

#[test]
fn transitions_can_be_removed_while_transition_in_progress() {
    start();
    let transitions = opacity_transitions();
    let border_theme = ControlTheme::for_type::<Border>();
    border_theme.add_setter(Setter::new(Animatable::transitions_property(), Some(transitions.clone())));

    let clock = TestClock::new();
    let root = TestRoot::new();
    root.set_clock(Some(clock.as_clock()));
    let theme: BoxedValue = Rc::new(border_theme);
    root.resources().add(ResourceKey::Type(Border::TYPE), Some(theme));
    let border = Border::new();
    root.set_child(&border);

    assert_eq!(border.transitions(), Some(transitions));

    // First set property with a transition to a new value, and step the clock until
    // transition is complete.
    border.set_opacity(0.0);
    clock.step(seconds(0.0));
    clock.step(seconds(1.0));
    assert_eq!(border.opacity(), 0.0);

    // Now clear the property; a transition is now in progress but no local value is
    // set.
    border.clear_value(Visual::opacity_property());

    // Remove the transition by removing the control from the logical tree. This was
    // causing an exception.
    root.remove_child(&border);
}

#[test]
fn zero_duration_should_finish_animation() {
    start();
    let animation = Animation::new();
    animation.set_duration(seconds(2.0));
    animation.set_fill_mode(FillMode::Both);
    animation.children().add(KeyFrame::with_key_time(seconds(0.0), [Setter::new(Visual::opacity_property(), 1.0) as _]));
    animation.children().add(KeyFrame::with_key_time(seconds(2.0), [Setter::new(Visual::opacity_property(), 0.5) as _]));
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    clock.step(seconds(1.0));

    assert!(target.is_animating(Visual::opacity_property()));
    assert_eq!(target.opacity(), 0.75);

    // This is not the normal way to access and set the animations
    // object's Duration property to zero that is defined in styles
    // but this is still valid for the RunAsync version.
    animation.set_duration(TimeSpan::ZERO);

    clock.step(seconds(1.2));

    assert_eq!(target.opacity(), 0.5);
    assert!(!target.is_animating(Visual::opacity_property()));
}

#[test]
fn zero_duration_should_finish_animation_with_infinite_iteration() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_iteration_count(IterationCount::INFINITE);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    assert!(target.is_animating(Visual::opacity_property()));

    for (time, expected) in [(0.5, 0.5), (1.0, 0.0), (1.5, 0.5), (2.0, 0.0)] {
        clock.step(seconds(time));
        assert_eq!(target.opacity(), expected, "at {time}");
    }

    animation.set_duration(TimeSpan::ZERO);

    clock.step(seconds(1.2));

    assert_eq!(target.opacity(), 1.0);
    assert!(!target.is_animating(Visual::opacity_property()));
}

#[test]
fn changing_playback_direction_keeps_playback_time_constant() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_playback_direction(PlaybackDirection::Normal);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));

    clock.step(seconds(0.25));
    assert_eq!(target.opacity(), 0.25);

    clock.step(seconds(0.3));
    assert_eq!(target.opacity(), 0.3);
    animation.set_playback_direction(PlaybackDirection::Reverse);
    clock.step(seconds(0.3));
    assert_eq!(target.opacity(), 0.3);

    clock.step(seconds(0.35));
    assert_eq!(target.opacity(), 0.25);
}

#[test]
fn delay_between_iterations_behind_initial_point_is_in_front_of_iterations() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_delay_between_iterations(seconds(0.5));
    animation.set_iteration_count(IterationCount::INFINITE);
    animation.set_playback_direction(PlaybackDirection::Normal);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    assert_eq!(target.opacity(), 0.0);

    clock.step(seconds(0.1));
    assert_eq!(target.opacity(), 0.1);
    animation.set_playback_direction(PlaybackDirection::Reverse);
    clock.step(seconds(0.1));
    assert_eq!(target.opacity(), 0.1);

    clock.step(seconds(0.2));
    assert_eq!(target.opacity(), 0.0);

    clock.step(seconds(0.2 + 0.1));
    assert_eq!(target.opacity(), 0.9);

    clock.step(seconds(0.2 + 0.9));
    assert_close(target.opacity(), 0.1, 0.000001);

    clock.step(seconds(0.2 + 1.0));
    assert_eq!(target.opacity(), 0.0);

    clock.step(seconds(0.2 + 1.0 + 0.25));
    assert_eq!(target.opacity(), 0.0);

    clock.step(seconds(0.2 + 1.0 + 0.499));
    assert_eq!(target.opacity(), 0.0);

    clock.step(seconds(0.2 + 1.0 + 0.5));
    assert_eq!(target.opacity(), 1.0);

    clock.step(seconds(0.2 + 1.0 + 0.5 + 0.1));
    assert_eq!(target.opacity(), 0.9);
}

#[test]
fn zero_duration_with_delay_between_iterations_should_finish_animation() {
    start();
    let animation = Animation::new();
    animation.set_duration(TimeSpan::ZERO);
    animation.set_delay_between_iterations(seconds(0.5));
    animation.set_iteration_count(IterationCount::INFINITE);
    animation
        .children()
        .add(KeyFrame::with_cue(crate::animation::Cue::new(0.0), [Setter::new(Visual::opacity_property(), 0.0) as _]));
    animation
        .children()
        .add(KeyFrame::with_cue(crate::animation::Cue::new(1.0), [Setter::new(Visual::opacity_property(), 1.0) as _]));
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));

    assert!(!target.opacity().is_nan());
    assert_eq!(target.opacity(), 1.0);
    assert!(!target.is_animating(Visual::opacity_property()));

    clock.step(seconds(0.5));

    assert!(!target.opacity().is_nan());
}

#[test]
fn setting_duration_to_zero_with_delay_between_iterations_should_finish_animation() {
    start();
    let animation = opacity_animation(0.0, 1.0, 1.0);
    animation.set_delay_between_iterations(seconds(0.5));
    animation.set_iteration_count(IterationCount::INFINITE);
    let (clock, _root, target) = animated_border(&animation);

    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    animation.set_duration(TimeSpan::ZERO);
    clock.step(seconds(1.0));

    assert!(!target.opacity().is_nan());
    assert_eq!(target.opacity(), 1.0);
    assert!(!target.is_animating(Visual::opacity_property()));
}

#[test]
fn reversing_direction_past_initial_point_parks_animation_until_resumed() {
    let animation = opacity_animation(0.0, 1.0, 1.0);

    let target = Border::new();
    target.set_background(Some(Brushes::red()));
    let clock = TestClock::new();
    let animation_run = animation.run_async_with_clock(&target, Some(clock.as_clock()), CancellationToken::none());

    clock.step(seconds(0.0));
    clock.step(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    // Reverse the animation so that it runs back to its initial point, where a limited
    // number of iterations clamps it. The animation is parked rather than finished: it
    // stays subscribed, and so the run stays pending, because playback can still be
    // resumed by changing the direction back.
    animation.set_playback_direction(PlaybackDirection::Reverse);
    clock.step(seconds(0.5));
    clock.step(seconds(1.0));
    clock.step(seconds(10.0));
    assert_eq!(target.opacity(), 0.0);
    assert!(!animation_run.is_completed());

    // Resuming from the parked state runs the animation to its end and completes it.
    animation.set_playback_direction(PlaybackDirection::Normal);
    clock.step(seconds(10.0));
    clock.step(seconds(10.5));
    assert_eq!(target.opacity(), 0.5);

    clock.step(seconds(11.0));
    assert_eq!(target.opacity(), 1.0);
    assert!(animation_run.is_completed());
}

#[test]
fn try_get_transition_instance_returns_the_running_instance() {
    start();
    let clock = TestClock::new();
    let transition = DoubleTransition::new();
    transition.set_property(Some(Visual::opacity_property()));
    transition.set_duration(seconds(1.0));
    let handle: Rc<dyn crate::animation::ITransition> = (&transition).into();
    let border = Border::new();
    border.set_transitions(Some(Transitions::from_items([handle.clone()])));
    let root = TestRoot::with_child(&border);
    root.set_clock(Some(clock.as_clock()));

    assert!(border.try_get_transition_instance(&handle).is_none());

    border.set_opacity(0.0);
    let first = border.try_get_transition_instance(&handle).expect("a running transition");

    border.set_opacity(0.5);
    let second = border.try_get_transition_instance(&handle).expect("a running transition");
    assert!(!Rc::ptr_eq(&first, &second));

    let other: Rc<dyn crate::animation::ITransition> = DoubleTransition::new().into();
    assert!(border.try_get_transition_instance(&other).is_none());

    border.transitions().unwrap().clear();
    assert!(border.try_get_transition_instance(&handle).is_none());
}
