//! Tests of the storyboards (#443, VSM-3), against the behaviour of WinUI's `CStoryboard`,
//! `CTimeline`, `CAnimation` and the easing functions.

use super::*;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::test_scope;
use crate::{Border, ContentControl, VisualState, VisualStateGroup, VisualStateManager};
use ferroui_base::animation::{
    ClockState, FillBehavior, IClock, IGlobalClock, PlayState, RepeatBehavior, SignedDuration, TimelineDuration,
};
use ferroui_base::media::{Color, Colors, IBrush, SolidColorBrush};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{BoxedValue, FerroLocator, Ref, Visual};
use std::cell::Cell;
use std::rc::Rc;

fn ms(value: f64) -> SignedDuration {
    SignedDuration::from_milliseconds(value)
}

fn seconds(value: f64) -> TimelineDuration {
    TimelineDuration::Time(std::time::Duration::from_secs_f64(value))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// A double animation of the opacity of `target` from `from` to `to` over a second.
fn opacity_animation(target: &Border, from: Option<f64>, to: Option<f64>) -> Ref<DoubleAnimation> {
    let animation = DoubleAnimation::new();
    animation.set_from(from);
    animation.set_to(to);
    animation.set_duration(seconds(1.0));
    Storyboard::set_target(&animation, target);
    Storyboard::set_target_property(&animation, "Opacity");
    animation
}

fn storyboard_of(timelines: &[Ref<Timeline>]) -> Ref<Storyboard> {
    let storyboard = Storyboard::new();
    for timeline in timelines {
        storyboard.children().add(timeline.clone());
    }
    storyboard
}

#[test]
fn easing_functions_are_those_of_winui() {
    let cubic = CubicEase::with_mode(EasingMode::EaseIn);
    assert!(close(cubic.ease(0.5), 0.125));
    cubic.set_easing_mode(EasingMode::EaseOut);
    assert!(close(cubic.ease(0.5), 0.875));
    cubic.set_easing_mode(EasingMode::EaseInOut);
    assert!(close(cubic.ease(0.25), 0.0625));
    assert!(close(cubic.ease(0.75), 0.9375));
    // The default mode is EaseOut.
    assert_eq!(QuadraticEase::new().easing_mode(), EasingMode::EaseOut);
    assert!(close(QuadraticEase::new().ease(0.5), 0.75));
    // Every function goes from 0 to 1.
    let functions: Vec<Ref<EasingFunctionBase>> = vec![
        BackEase::new().upcast(),
        BounceEase::new().upcast(),
        CircleEase::new().upcast(),
        CubicEase::new().upcast(),
        ElasticEase::new().upcast(),
        ExponentialEase::new().upcast(),
        PowerEase::new().upcast(),
        QuadraticEase::new().upcast(),
        QuarticEase::new().upcast(),
        QuinticEase::new().upcast(),
        SineEase::new().upcast(),
    ];
    for function in &functions {
        for mode in [EasingMode::EaseIn, EasingMode::EaseOut, EasingMode::EaseInOut] {
            function.set_easing_mode(mode);
            assert!(
                function.ease(0.0).abs() < 1e-3,
                "{:?} {mode:?}",
                function.get_type().name()
            );
            assert!(
                (function.ease(1.0) - 1.0).abs() < 1e-3,
                "{:?} {mode:?}",
                function.get_type().name()
            );
        }
    }
    // The parameters: a negative power is 0 (a constant 1 when eased in), an exponent of 0 is
    // linear.
    let power = PowerEase::with_mode(EasingMode::EaseIn);
    power.set_power(-2.0);
    assert!(close(power.ease(0.3), 1.0));
    let exponential = ExponentialEase::with_mode(EasingMode::EaseIn);
    exponential.set_exponent(0.0);
    assert!(close(exponential.ease(0.3), 0.3));
    // The back ease pulls back below 0 when eased in.
    assert!(BackEase::with_mode(EasingMode::EaseIn).ease(0.3) < 0.0);
    // As an easing of the framework, with the same curve.
    let easing = CubicEase::with_mode(EasingMode::EaseIn).to_easing();
    assert!(close(easing.ease(0.5), 0.125));
}

#[test]
fn a_double_animation_animates_above_the_local_value_and_stop_removes_it() {
    let _scope = test_scope();
    let border = Border::new();
    border.set_opacity(0.3);
    let storyboard = storyboard_of(&[opacity_animation(&border, Some(0.0), Some(1.0)).upcast()]);
    let completed = Rc::new(Cell::new(0));
    let count = completed.clone();
    storyboard.completed(move |_| count.set(count.get() + 1)).detach();

    storyboard.begin();
    // The value of the start applies at once.
    assert!(close(border.opacity(), 0.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Active);
    storyboard.seek(ms(250.0));
    assert!(close(border.opacity(), 0.25));
    // The local value is not changed.
    assert_eq!(border.get_base_value(Visual::opacity_property()), Some(0.3));
    // After the end: the end value is held, and Completed is raised once.
    storyboard.seek(ms(1500.0));
    assert!(close(border.opacity(), 1.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Filling);
    assert_eq!(completed.get(), 1);
    storyboard.skip_to_fill();
    assert_eq!(completed.get(), 1);
    storyboard.stop();
    assert!(close(border.opacity(), 0.3));
    assert_eq!(storyboard.get_current_state(), ClockState::Stopped);
}

#[test]
fn from_to_and_by_follow_winui() {
    let _scope = test_scope();
    let border = Border::new();
    border.set_opacity(0.4);
    // To alone: from the current value.
    let to = storyboard_of(&[opacity_animation(&border, None, Some(1.0)).upcast()]);
    to.begin();
    to.seek(ms(500.0));
    assert!(close(border.opacity(), 0.7));
    to.stop();
    // By: from the start, by the change.
    let by = opacity_animation(&border, Some(0.2), None);
    by.set_by(Some(0.5));
    let by = storyboard_of(&[by.upcast()]);
    by.begin();
    by.seek(ms(1000.0));
    assert!(close(border.opacity(), 0.7));
    by.stop();
    // From alone: back to the value without animations.
    let from = storyboard_of(&[opacity_animation(&border, Some(0.0), None).upcast()]);
    from.begin();
    from.seek(ms(500.0));
    assert!(close(border.opacity(), 0.2));
    from.stop();
    assert!(close(border.opacity(), 0.4));
}

#[test]
fn fill_behavior_stop_removes_the_value_at_the_end() {
    let _scope = test_scope();
    let border = Border::new();
    let animation = opacity_animation(&border, Some(0.0), Some(0.5));
    animation.set_fill_behavior(FillBehavior::Stop);
    let storyboard = storyboard_of(&[animation.upcast()]);
    storyboard.begin();
    storyboard.seek(ms(500.0));
    assert!(close(border.opacity(), 0.25));
    storyboard.seek(ms(2000.0));
    assert!(close(border.opacity(), 1.0));
}

#[test]
fn begin_times_repeats_and_auto_reverse_apply_to_the_animations() {
    let _scope = test_scope();
    let border = Border::new();
    let animation = opacity_animation(&border, Some(0.0), Some(1.0));
    animation.set_begin_time(Some(ms(500.0)));
    animation.set_auto_reverse(true);
    animation.set_repeat_behavior(RepeatBehavior::Count(2.0));
    let storyboard = storyboard_of(&[animation.clone().upcast()]);
    // The storyboard lasts until the end of its child: 0.5 + 2 x 2 seconds.
    storyboard.begin();
    // Before the begin time: no effect.
    assert!(close(border.opacity(), 1.0));
    storyboard.seek(ms(750.0));
    assert!(close(border.opacity(), 0.25));
    // In the reverse half.
    storyboard.seek(ms(1750.0));
    assert!(close(border.opacity(), 0.75));
    storyboard.seek(ms(4000.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Active);
    storyboard.seek(ms(4600.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Filling);
    // An auto-reversed animation ends at its start.
    assert!(close(border.opacity(), 0.0));
}

#[test]
fn key_frames_interpolate_their_segments() {
    let _scope = test_scope();
    let border = Border::new();
    let animation = DoubleAnimationUsingKeyFrames::new();
    // In any order: sorted by key time.
    animation
        .key_frames()
        .add(LinearDoubleKeyFrame::with(ms(1000.0), 1.0).upcast());
    animation
        .key_frames()
        .add(DiscreteDoubleKeyFrame::with(ms(0.0), 0.2).upcast());
    let easing = EasingDoubleKeyFrame::new();
    easing.set_key_time(ms(2000.0));
    easing.set_frame_value(0.0);
    easing.set_easing_function(Some(CubicEase::with_mode(EasingMode::EaseIn).upcast()));
    animation.key_frames().add(easing.upcast());
    Storyboard::set_target(&animation, &border);
    Storyboard::set_target_property(&animation, "Opacity");
    let storyboard = storyboard_of(&[animation.upcast()]);
    storyboard.begin();
    assert!(close(border.opacity(), 0.2));
    storyboard.seek(ms(500.0));
    assert!(close(border.opacity(), 0.6));
    // The eased segment: from 1 to 0 with t^3.
    storyboard.seek(ms(1500.0));
    assert!(close(border.opacity(), 1.0 - 0.125));
    // The automatic duration is the last key time.
    storyboard.seek(ms(2500.0));
    assert!(close(border.opacity(), 0.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Filling);
}

#[test]
fn object_key_frames_convert_text_to_the_type_of_the_property() {
    let _scope = test_scope();
    let border = Border::new();
    let animation = ObjectAnimationUsingKeyFrames::new();
    let text: BoxedValue = Rc::new("False".to_string());
    animation
        .key_frames()
        .add(DiscreteObjectKeyFrame::with(ms(100.0), Some(text)).upcast());
    Storyboard::set_target(&animation, &border);
    Storyboard::set_target_property(&animation, "IsVisible");
    let storyboard = storyboard_of(&[animation.upcast()]);
    storyboard.begin();
    // Before the key frame: the value the property had.
    assert!(border.is_visible());
    storyboard.seek(ms(100.0));
    assert!(!border.is_visible());
    storyboard.stop();
    assert!(border.is_visible());
}

#[test]
fn colour_animations_interpolate_the_channels_and_paths_reach_into_brushes() {
    let _scope = test_scope();
    let border = Border::new();
    let brush = SolidColorBrush::with_color(Colors::BLACK);
    let handle: Rc<dyn IBrush> = brush.clone().into();
    border.set_background(Some(handle));
    let animation = ColorAnimation::new();
    animation.set_to(Some(Color::new(255, 200, 100, 0)));
    animation.set_duration(seconds(1.0));
    Storyboard::set_target(&animation, &border);
    Storyboard::set_target_property(&animation, "(Border.Background).(SolidColorBrush.Color)");
    let storyboard = storyboard_of(&[animation.upcast()]);
    storyboard.begin();
    storyboard.seek(ms(500.0));
    assert_eq!(brush.color(), Color::new(255, 100, 50, 0));
    storyboard.stop();
    assert_eq!(brush.color(), Colors::BLACK);
}

#[test]
fn property_paths_parse_owners_and_chains() {
    assert_eq!(
        property_path::parse("Opacity").unwrap(),
        vec![(String::new(), "Opacity".to_string())]
    );
    assert_eq!(
        property_path::parse("(Visual.RenderTransform).(ScaleTransform.ScaleX)").unwrap(),
        vec![
            ("Visual".to_string(), "RenderTransform".to_string()),
            ("ScaleTransform".to_string(), "ScaleX".to_string())
        ]
    );
    assert!(property_path::parse("").is_err());
    assert!(property_path::parse("(Grid.Row").is_err());
    assert!(property_path::parse("(Row)").is_err());
    assert!(property_path::parse("Opacity.").is_err());
    let _scope = test_scope();
    let border = Border::new();
    let target: Ref<ferroui_base::FerroObject> = border.upcast();
    let (object, property) = property_path::resolve(&target, "(Visual.Opacity)").unwrap();
    assert_eq!(object, target);
    assert_eq!(property, Visual::opacity_property().as_property());
    assert!(property_path::resolve(&target, "(Visual.NoSuchProperty)").is_err());
}

#[test]
fn nested_storyboards_offset_their_children_and_pass_on_their_targets() {
    let _scope = test_scope();
    let border = Border::new();
    let inner = Storyboard::new();
    inner.set_begin_time(Some(ms(1000.0)));
    Storyboard::set_target(&inner, &border);
    let animation = DoubleAnimation::new();
    animation.set_from(Some(0.0));
    animation.set_to(Some(1.0));
    animation.set_duration(seconds(1.0));
    Storyboard::set_target_property(&animation, "Opacity");
    inner.children().add(animation.upcast());
    let outer = storyboard_of(&[inner.upcast()]);
    outer.begin();
    assert!(close(border.opacity(), 1.0));
    outer.seek(ms(1500.0));
    assert!(close(border.opacity(), 0.5));
    outer.seek(ms(3000.0));
    assert_eq!(outer.get_current_state(), ClockState::Filling);
}

/// A global clock that the test pulses.
struct TestClock {
    subject: LightweightSubject<SignedDuration>,
    play_state: Cell<PlayState>,
}

impl IObservable<SignedDuration> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for TestClock {}

#[test]
fn a_storyboard_runs_on_the_global_clock_and_pauses() {
    let _scope = test_scope();
    let clock = Rc::new(TestClock {
        subject: LightweightSubject::new(),
        play_state: Cell::new(PlayState::Run),
    });
    let service: Rc<dyn IGlobalClock> = clock.clone();
    FerroLocator::current_mutable()
        .bind::<dyn IGlobalClock>()
        .to_constant(service);
    let border = Border::new();
    let storyboard = storyboard_of(&[opacity_animation(&border, Some(0.0), Some(1.0)).upcast()]);
    storyboard.begin();
    assert!(clock.subject.has_observers());
    // The first tick starts the time.
    clock.subject.on_next(ms(10_000.0));
    clock.subject.on_next(ms(10_250.0));
    assert!(close(border.opacity(), 0.25));
    storyboard.pause();
    clock.subject.on_next(ms(10_500.0));
    assert!(close(border.opacity(), 0.25));
    storyboard.resume();
    clock.subject.on_next(ms(10_750.0));
    assert!(close(border.opacity(), 0.5));
    // At the end the clock is not followed any more.
    clock.subject.on_next(ms(12_000.0));
    assert!(close(border.opacity(), 1.0));
    assert!(!clock.subject.has_observers());
    assert!(storyboard.get_current_time() >= ms(1000.0));
}

#[test]
fn the_storyboard_of_a_visual_state_runs_while_the_state_is_current() {
    let _scope = test_scope();
    let control = ContentControl::new();
    let states: Rc<std::cell::RefCell<Option<Ref<VisualStateGroup>>>> = Rc::default();
    let group_slot = states.clone();
    control.set_template(Some(FuncControlTemplate::new(move |_, scope| {
        let inner = Border::new();
        inner.set_name(Some("Inner".to_string()));
        let root = Border::new();
        root.set_child(inner.clone().register_in_name_scope(&**scope));
        let group = VisualStateGroup::with_name("CommonStates");
        group.states().add(VisualState::with_name("Normal"));
        let faded = VisualState::with_name("Faded");
        let animation = DoubleAnimation::new();
        animation.set_to(Some(0.5));
        animation.set_duration(TimelineDuration::Time(std::time::Duration::ZERO));
        Storyboard::set_target_name(&animation, "Inner");
        Storyboard::set_target_property(&animation, "Opacity");
        let storyboard = Storyboard::new();
        storyboard.children().add(animation.upcast());
        faded.set_storyboard(Some(storyboard));
        group.states().add(faded);
        VisualStateManager::get_visual_state_groups(&root).add(group.clone());
        *group_slot.borrow_mut() = Some(group);
        root.upcast()
    })));
    control.apply_template();
    let root = control.visual_children().snapshot()[0].cast::<Border>().unwrap();
    let inner = root.child().unwrap().cast::<Border>().unwrap();

    assert!(VisualStateManager::go_to_state(&control, "Faded", false));
    assert!(close(inner.opacity(), 0.5));
    let storyboard = states.borrow().as_ref().unwrap().states().snapshot()[1]
        .storyboard()
        .unwrap();
    assert_eq!(storyboard.get_current_state(), ClockState::Filling);
    assert!(VisualStateManager::go_to_state(&control, "Normal", false));
    assert!(close(inner.opacity(), 1.0));
    assert_eq!(storyboard.get_current_state(), ClockState::Stopped);
}
