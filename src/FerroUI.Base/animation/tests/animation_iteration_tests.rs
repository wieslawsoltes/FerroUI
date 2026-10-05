use super::*;
use crate::animation::easings::{LinearEasing, SineEaseInOut};
use crate::media::{ScaleTransform, TransformGroup};
use crate::styling::SetterValue;
use crate::animation::{
    Animation, AnimationTask, Cue, FillMode, InterpolatingAnimator, IterationCount, KeyFrame, PlaybackBehavior,
};
use crate::reactive::Observable;
use crate::styling::Setter;
use crate::threading::{CancellationToken, CancellationTokenSource, Dispatcher};

fn width_key_frame(width: f64, cue: f64) -> Ref<KeyFrame> {
    KeyFrame::with_cue(Cue::new(cue), [Setter::new(Layoutable::width_property(), width) as _])
}

fn border(width: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_height(100.0);
    border.set_width(width);
    border
}

fn run(animation: &Ref<Animation>, border: &Ref<Border>, clock: &Rc<TestClock>) -> AnimationTask {
    animation.run_async_with_clock(border, Some(clock.as_clock()), CancellationToken::none())
}

#[test]
fn check_key_time_correctly_converted_to_cue() {
    let keyframe1 = KeyFrame::with_key_time(seconds(0.5), [Setter::new(Layoutable::width_property(), 100.0) as _]);
    let keyframe2 = KeyFrame::with_key_time(seconds(0.0), [Setter::new(Layoutable::width_property(), 0.0) as _]);

    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.children().add(keyframe2);
    animation.children().add(keyframe1);

    let border = border(100.0);
    let clock = TestClock::new();
    run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert_eq!(border.width(), 0.0);

    clock.step(seconds(1.0));
    assert_eq!(border.width(), 100.0);
}

#[test]
fn check_initial_inter_and_trailing_delay_values() {
    let animation = Animation::new();
    animation.set_duration(seconds(3.0));
    animation.set_delay(seconds(3.0));
    animation.set_delay_between_iterations(seconds(3.0));
    animation.set_iteration_count(IterationCount::new(2));
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));

    let border = border(100.0);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);

    // Initial Delay.
    clock.step(seconds(0.0));
    assert_eq!(border.width(), 100.0);

    clock.step(seconds(6.0));

    // First Inter-Iteration delay.
    clock.step(seconds(8.0));
    assert_eq!(border.width(), 200.0);

    // Trailing Delay should be non-existent.
    clock.step(seconds(14.0));
    assert_eq!(animation_run.result(), Some(Ok(())));
    assert_eq!(border.width(), 100.0);
}

#[test]
fn only_if_visible_pauses_animation_when_is_effectively_visible_is_false() {
    let animation = Animation::new();
    animation.set_duration(seconds(3.0));
    animation.set_delay(seconds(3.0));
    animation.set_delay_between_iterations(seconds(3.0));
    animation.set_iteration_count(IterationCount::new(2));
    // Explicit opt-in: a manual run with `Auto` resolves to `Always`, but
    // this test specifically exercises the pause-on-invisible feature.
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.children().add(width_key_frame(200.0, 1.0));
    animation.children().add(width_key_frame(150.0, 0.5));
    animation.children().add(width_key_frame(100.0, 0.0));

    let border = border(100.0);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    clock.step(seconds(0.0));
    assert_eq!(border.width(), 100.0);

    // Hide the border — this should pause the animation clock.
    border.set_is_visible(false);
    clock.step(seconds(4.5));

    // Width should not change while invisible (animation is paused).
    assert_eq!(border.width(), 100.0);

    // Show the border — animation resumes from where it left off.
    border.set_is_visible(true);

    // The pause absorbed 4.5s of wall-clock time, so internal time = wall - 4.5.
    clock.step(seconds(10.5));
    assert_eq!(border.width(), 200.0);

    clock.step(seconds(18.5));
    assert_eq!(animation_run.result(), Some(Ok(())));
    assert_eq!(border.width(), 100.0);
}

#[test]
fn stop_and_dispose_animation_when_detached_from_visual_tree() {
    let animation = Animation::new();
    animation.set_duration(seconds(10.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));

    let border = border(50.0);
    let root = TestRoot::with_child(&border);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    clock.step(seconds(0.0));
    assert!(!animation_run.is_completed());

    // Detach from visual tree
    root.remove_child(&border);

    // Animation should be completed/disposed
    assert!(animation_run.is_completed());

    // Further clock ticks should not affect the border
    let width_after_detach = border.width();
    clock.step(seconds(5.0));
    clock.step(seconds(10.0));
    assert_eq!(border.width(), width_after_detach);
}

#[test]
fn check_fill_modes_start_and_end_values_if_retained() {
    let animation = Animation::new();
    animation.set_duration(seconds(0.05));
    animation.set_delay(seconds(0.05));
    animation.set_easing(SineEaseInOut::new());
    animation.set_fill_mode(FillMode::Both);
    animation.children().add(width_key_frame(0.0, 0.0));
    animation.children().add(width_key_frame(300.0, 1.0));

    let border = border(100.0);
    let clock = TestClock::new();
    run(&animation, &border, &clock);

    clock.step(seconds(0.0));
    assert_eq!(border.width(), 0.0);

    clock.step(seconds(0.050));
    assert_eq!(border.width(), 0.0);

    clock.step(seconds(0.100));
    assert_eq!(border.width(), 300.0);
}

#[test]
fn check_fill_mode_start_value() {
    for (fill_mode, target, start_cue, end_cue, delay) in [
        (FillMode::Backward, 50.0, 0.0, 0.7, false),
        (FillMode::Backward, 50.0, 0.0, 0.7, true),
        (FillMode::Both, 50.0, 0.0, 0.7, false),
        (FillMode::Both, 50.0, 0.0, 0.7, true),
        // no delay but cue 0.0: the animation has started normally, explaining the 50.0 target without fill
        (FillMode::Forward, 50.0, 0.0, 0.7, false),
        (FillMode::Forward, 100.0, 0.0, 0.7, true),
        (FillMode::Backward, 50.0, 0.3, 0.7, false),
        (FillMode::Backward, 50.0, 0.3, 0.7, true),
        (FillMode::Both, 50.0, 0.3, 0.7, false),
        (FillMode::Both, 50.0, 0.3, 0.7, true),
        (FillMode::Forward, 100.0, 0.3, 0.7, false),
        (FillMode::Forward, 100.0, 0.3, 0.7, true),
    ] {
        let animation = Animation::new();
        animation.set_duration(seconds(10.0));
        animation.set_delay(if delay { seconds(5.0) } else { TimeSpan::ZERO });
        animation.set_fill_mode(fill_mode);
        animation.children().add(width_key_frame(50.0, start_cue));
        animation.children().add(width_key_frame(300.0, end_cue));

        let border = border(100.0);
        let clock = TestClock::new();
        run(&animation, &border, &clock);

        clock.step(TimeSpan::ZERO);

        assert_eq!(border.width(), target, "{fill_mode:?} {start_cue} {end_cue} {delay}");
    }
}

#[test]
fn check_fill_mode_end_value() {
    for (fill_mode, target, start_cue, end_cue, delay) in [
        (FillMode::Backward, 100.0, 0.3, 1.0, false),
        (FillMode::Backward, 100.0, 0.3, 1.0, true),
        (FillMode::Both, 300.0, 0.3, 1.0, false),
        (FillMode::Both, 300.0, 0.3, 1.0, true),
        (FillMode::Forward, 300.0, 0.3, 1.0, false),
        (FillMode::Forward, 300.0, 0.3, 1.0, true),
        (FillMode::Backward, 100.0, 0.3, 0.7, false),
        (FillMode::Backward, 100.0, 0.3, 0.7, true),
        (FillMode::Both, 300.0, 0.3, 0.7, false),
        (FillMode::Both, 300.0, 0.3, 0.7, true),
        (FillMode::Forward, 300.0, 0.3, 0.7, false),
        (FillMode::Forward, 300.0, 0.3, 0.7, true),
    ] {
        let animation = Animation::new();
        animation.set_duration(seconds(10.0));
        animation.set_delay(if delay { seconds(5.0) } else { TimeSpan::ZERO });
        animation.set_fill_mode(fill_mode);
        animation.children().add(width_key_frame(0.0, start_cue));
        animation.children().add(width_key_frame(300.0, end_cue));

        let border = border(100.0);
        let clock = TestClock::new();
        run(&animation, &border, &clock);

        clock.step(seconds(0.0));
        clock.step(seconds(20.0));

        assert_eq!(border.width(), target, "{fill_mode:?} {start_cue} {end_cue} {delay}");
    }
}

fn ten_second_width_animation() -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(10.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));
    animation
}

fn count_width_changes(border: &Ref<Border>) -> Rc<Cell<u32>> {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    border.property_changed(move |e| {
        if e.property() == Layoutable::width_property().as_property() {
            c.set(c.get() + 1);
        }
    });
    count
}

#[test]
fn dispose_subscription_should_stop_animation() {
    let animation = ten_second_width_animation();
    let border = border(50.0);
    let property_changed_count = count_width_changes(&border);

    let clock = TestClock::new();
    let completed = Rc::new(Cell::new(false));
    let c = completed.clone();
    let animation_run = animation.apply(
        &border,
        Some(clock.as_clock()),
        Observable::return_(true),
        Some(Rc::new(move || c.set(true))),
        false,
    );

    assert_eq!(property_changed_count.get(), 0);

    clock.step(seconds(0.0));
    assert_eq!(property_changed_count.get(), 1);

    animation_run.dispose();

    // Clock ticks should be ignored after the dispose.
    clock.step(seconds(5.0));
    clock.step(seconds(6.0));
    clock.step(seconds(7.0));

    // On animation disposal the value is reset.
    assert_eq!(property_changed_count.get(), 2);
    assert_eq!(border.width(), 50.0);
    assert!(!completed.get());
}

#[test]
fn do_not_run_cancelled_animation() {
    let animation = ten_second_width_animation();
    let border = border(50.0);
    let property_changed_count = count_width_changes(&border);

    let clock = TestClock::new();
    let cancellation_token_source = CancellationTokenSource::new();
    cancellation_token_source.cancel();
    let animation_run =
        animation.run_async_with_clock(&border, Some(clock.as_clock()), cancellation_token_source.token());

    clock.step(seconds(10.0));
    assert!(animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 0);
    assert!(!clock.has_observer());
}

#[test]
fn cancellation_should_stop_animation() {
    let animation = ten_second_width_animation();
    let border = border(50.0);
    let property_changed_count = count_width_changes(&border);

    let clock = TestClock::new();
    let cancellation_token_source = CancellationTokenSource::new();
    let animation_run =
        animation.run_async_with_clock(&border, Some(clock.as_clock()), cancellation_token_source.token());

    assert!(!animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 0);

    clock.step(seconds(0.0));
    assert!(!animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 1);

    cancellation_token_source.cancel();
    clock.step(seconds(1.0));
    clock.step(seconds(2.0));
    clock.step(seconds(3.0));
    assert_eq!(animation_run.result(), Some(Ok(())));

    clock.step(seconds(6.0));
    assert!(animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 2);
}

#[test]
fn cancellation_from_another_thread_stops_animation_on_the_dispatcher() {
    let animation = ten_second_width_animation();
    let border = border(50.0);

    let clock = TestClock::new();
    let cancellation_token_source = CancellationTokenSource::new();
    let animation_run =
        animation.run_async_with_clock(&border, Some(clock.as_clock()), cancellation_token_source.token());
    clock.step(seconds(0.0));
    assert_eq!(border.width(), 100.0);

    let source = cancellation_token_source.clone();
    std::thread::spawn(move || source.cancel()).join().unwrap();

    // The cancellation is carried out by the thread that owns the animation.
    assert!(!animation_run.is_completed());
    Dispatcher::current_dispatcher().run_jobs(None);
    assert!(animation_run.is_completed());
    assert_eq!(border.width(), 50.0);
}

#[test]
fn dont_run_infinite_iteration_animation_on_run_async_method() {
    let animation = ten_second_width_animation();
    animation.set_iteration_count(IterationCount::INFINITE);
    let border = border(50.0);

    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    assert!(animation_run.is_completed());
    assert!(matches!(animation_run.result(), Some(Err(_))));
}

#[test]
fn cancellation_of_completed_animation_does_not_fail() {
    let animation = ten_second_width_animation();
    let border = border(50.0);
    let property_changed_count = count_width_changes(&border);

    let clock = TestClock::new();
    let cancellation_token_source = CancellationTokenSource::new();
    let animation_run =
        animation.run_async_with_clock(&border, Some(clock.as_clock()), cancellation_token_source.token());

    assert_eq!(property_changed_count.get(), 0);

    clock.step(seconds(0.0));
    assert!(!animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 1);

    clock.step(seconds(10.0));
    assert!(animation_run.is_completed());
    assert_eq!(property_changed_count.get(), 2);

    cancellation_token_source.cancel();
    assert!(animation_run.is_completed());
    assert_eq!(animation_run.result(), Some(Ok(())));
}

#[derive(Default)]
struct FakeAnimator;

thread_local! {
    static FAKE_CALL_COUNT: Cell<u32> = const { Cell::new(0) };
    static FAKE_LAST_PROGRESS: Cell<f64> = const { Cell::new(f64::NAN) };
}

impl InterpolatingAnimator for FakeAnimator {
    type Value = f64;

    fn interpolate(&self, progress: f64, _old_value: &f64, new_value: &f64) -> f64 {
        FAKE_CALL_COUNT.set(FAKE_CALL_COUNT.get() + 1);
        FAKE_LAST_PROGRESS.set(progress);
        *new_value
    }
}

#[test]
fn key_frames_order_does_not_matter() {
    for (index0, index1, index2) in [(0, 1, 2), (0, 2, 1), (1, 0, 2), (1, 2, 0), (2, 0, 1), (2, 1, 0)] {
        let key_frames = [width_key_frame(100.0, 0.0), width_key_frame(200.0, 0.5), width_key_frame(300.0, 1.0)];

        let animation = Animation::new();
        animation.set_duration(seconds(1.0));
        animation.set_iteration_count(IterationCount::new(1));
        animation.set_easing(LinearEasing::new());
        animation.set_fill_mode(FillMode::Forward);

        animation.children().add(key_frames[index0].clone());
        animation.children().add(key_frames[index1].clone());
        animation.children().add(key_frames[index2].clone());

        let border = border(50.0);
        let clock = TestClock::new();
        run(&animation, &border, &clock);

        clock.step(TimeSpan::ZERO);
        assert_eq!(border.width(), 100.0);

        clock.step(seconds(0.5));
        assert_eq!(border.width(), 200.0);

        clock.step(seconds(1.0));
        assert_eq!(border.width(), 300.0);
    }
}

#[test]
fn single_key_frame_works() {
    for cue in [0.0, 0.5, 1.0] {
        let animation = Animation::new();
        animation.set_duration(seconds(1.0));
        animation.set_iteration_count(IterationCount::new(1));
        animation.set_easing(LinearEasing::new());
        animation.set_fill_mode(FillMode::Forward);
        animation.children().add(width_key_frame(100.0, cue));

        let border = border(50.0);
        let clock = TestClock::new();
        run(&animation, &border, &clock);

        clock.step(TimeSpan::ZERO);
        clock.step(seconds(cue));

        assert_eq!(border.width(), 100.0);
    }
}

#[test]
fn completion_of_animation_with_several_properties_is_posted_to_the_dispatcher() {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(0.0),
        [Setter::new(Layoutable::width_property(), 100.0) as _, Setter::new(Visual::opacity_property(), 0.0) as _],
    ));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(1.0),
        [Setter::new(Layoutable::width_property(), 200.0) as _, Setter::new(Visual::opacity_property(), 1.0) as _],
    ));

    let border = border(50.0);
    let clock = MockGlobalClockHandle::new();
    let animation_run = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(0.5));
    assert_eq!(border.width(), 150.0);
    assert_eq!(border.opacity(), 0.5);

    clock.pulse(seconds(1.5));
    assert!(!animation_run.is_completed());
    Dispatcher::current_dispatcher().run_jobs(None);
    assert!(animation_run.is_completed());
}

/// A clock with any number of observers.
struct MockGlobalClockHandle;

impl MockGlobalClockHandle {
    fn new() -> Rc<MockGlobalClock> {
        start()
    }
}

trait AsClock {
    fn as_clock(&self) -> Rc<dyn crate::animation::IClock>;
}

impl AsClock for Rc<MockGlobalClock> {
    fn as_clock(&self) -> Rc<dyn crate::animation::IClock> {
        self.clone()
    }
}

#[test]
fn only_if_visible_resumes_transform_animations_with_visual() {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.set_iteration_count(IterationCount::INFINITE);
    animation.set_easing(LinearEasing::new());
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.children().add(KeyFrame::with_cue(
        Cue::new(0.0),
        [Setter::new(ScaleTransform::scale_x_property(), 1.0) as _, Setter::new(Visual::opacity_property(), 0.9) as _],
    ));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(1.0),
        [Setter::new(ScaleTransform::scale_x_property(), 2.5) as _, Setter::new(Visual::opacity_property(), 0.0) as _],
    ));

    let border = Border::new();
    let clock = start();
    let subscription = animation.apply(&border, Some(clock.as_clock()), Observable::return_(true), None, false);

    clock.pulse(seconds(0.25));

    let render_transform = border.render_transform().expect("a render transform");
    let group = render_transform.as_object().and_then(|o| o.downcast_ref::<TransformGroup>()).expect("a group");
    let scale = group.children().get(0).cast::<ScaleTransform>().expect("a scale transform");
    let scale_before_pause = scale.scale_x();
    let opacity_before_pause = border.opacity();

    border.set_is_visible(false);
    clock.pulse(seconds(0.75));

    assert_eq!(scale.scale_x(), scale_before_pause);
    assert_eq!(border.opacity(), opacity_before_pause);

    border.set_is_visible(true);
    clock.pulse(seconds(1.0));

    assert_eq!(scale.scale_x(), 1.375);
    assert_close(border.opacity(), 0.675, 1e-10);

    subscription.dispose();
}

#[test]
fn only_if_visible_pauses_animation_when_is_effectively_visible_is_false_nested() {
    let animation = Animation::new();
    animation.set_duration(seconds(3.0));
    animation.set_delay(seconds(3.0));
    animation.set_delay_between_iterations(seconds(3.0));
    animation.set_iteration_count(IterationCount::new(2));
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.children().add(width_key_frame(200.0, 1.0));
    animation.children().add(width_key_frame(150.0, 0.5));
    animation.children().add(width_key_frame(100.0, 0.0));

    let border = border(100.0);
    let border_parent = Border::new();
    add_child(&border_parent, &border);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    clock.step(seconds(0.0));
    assert_eq!(border.width(), 100.0);

    // Hide the parent — this makes the border not effectively visible,
    // which should pause the animation clock.
    border_parent.set_is_visible(false);
    clock.step(seconds(4.5));

    // Width should not change while parent is invisible (animation is paused).
    assert_eq!(border.width(), 100.0);

    // Show the parent — animation resumes from where it left off.
    border_parent.set_is_visible(true);

    clock.step(seconds(10.5));
    assert_eq!(border.width(), 200.0);

    clock.step(seconds(18.5));
    assert_eq!(animation_run.result(), Some(Ok(())));
    assert_eq!(border.width(), 100.0);
}

#[test]
fn only_if_visible_pauses_animation_when_control_starts_invisible() {
    let animation = Animation::new();
    animation.set_duration(seconds(3.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));

    let border = border(100.0);
    border.set_is_visible(false);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    // Clock ticks while invisible should not advance the animation.
    clock.step(TimeSpan::ZERO);
    clock.step(seconds(1.0));
    clock.step(seconds(2.0));
    assert_eq!(border.width(), 100.0);
    assert!(!animation_run.is_completed());

    // Make visible — animation starts from the beginning.
    border.set_is_visible(true);

    // The pause absorbed 2s of wall-clock time, so to reach internal time 3s:
    // wall = 2 + 3 = 5
    clock.step(seconds(5.0));
    assert!(animation_run.is_completed());
    assert_eq!(border.width(), 100.0);
}

#[test]
fn animation_plays_correctly_after_reattach() {
    let animation = Animation::new();
    animation.set_duration(seconds(5.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.set_fill_mode(FillMode::Forward);
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));

    let border = border(50.0);
    let root = TestRoot::with_child(&border);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert!(!animation_run.is_completed());

    // Detach — animation completes.
    root.remove_child(&border);
    assert!(animation_run.is_completed());

    // Reattach and start a fresh animation.
    root.set_child(&border);
    let clock2 = TestClock::new();
    let animation_run2 = run(&animation, &border, &clock2);

    clock2.step(TimeSpan::ZERO);
    assert!(!animation_run2.is_completed());

    clock2.step(seconds(5.0));
    assert!(animation_run2.is_completed());
    assert_eq!(border.width(), 200.0);
}

#[test]
fn interpolator_is_not_called_after_last_iteration() {
    let animator: Rc<dyn crate::animation::ICustomAnimator> = Rc::new(FakeAnimator);
    let create_width_setter = |value: f64| {
        let setter: Rc<dyn crate::animation::IAnimationSetter> = Setter::new(Layoutable::width_property(), value);
        Animation::set_animator(&setter, animator.clone());
        setter
    };

    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.set_delay(seconds(0.0));
    animation.set_delay_between_iterations(seconds(0.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.set_easing(LinearEasing::new());
    animation.children().add(KeyFrame::with_cue(Cue::new(0.0), [create_width_setter(100.0)]));
    animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [create_width_setter(200.0)]));

    let border = border(50.0);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert_eq!(FAKE_CALL_COUNT.get(), 1);
    assert_eq!(FAKE_LAST_PROGRESS.get(), 0.0);

    FAKE_LAST_PROGRESS.set(f64::NAN);
    clock.step(seconds(0.5));
    assert_eq!(FAKE_CALL_COUNT.get(), 2);
    assert_eq!(FAKE_LAST_PROGRESS.get(), 0.5);

    FAKE_LAST_PROGRESS.set(f64::NAN);
    clock.step(seconds(1.5));
    assert_eq!(FAKE_CALL_COUNT.get(), 3);
    assert_eq!(FAKE_LAST_PROGRESS.get(), 1.0);

    assert_eq!(animation_run.result(), Some(Ok(())));
}

/// A binding to a property that does not exist: it evaluates to nothing.
fn unresolved_binding(path: &str) -> SetterValue {
    SetterValue::BindingBase(Rc::new(crate::data::ReflectionBinding::new(path)))
}

#[test]
fn animation_completes_gracefully_when_first_key_frame_value_is_null() {
    let clock = start();
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.set_fill_mode(FillMode::Both);
    animation.children().add(KeyFrame::with_key_time(
        seconds(0.0),
        [Setter::with_value(Layoutable::width_property(), unresolved_binding("NonExistentProperty")) as _],
    ));
    animation
        .children()
        .add(KeyFrame::with_key_time(seconds(1.0), [Setter::new(Layoutable::width_property(), 200.0) as _]));

    let border = border(100.0);
    let _root = TestRoot::with_child(&border);

    let animation_task = animation.run_async_with_clock(&border, Some(clock.as_clock()), CancellationToken::none());

    // Pulse the clock - this should not panic even though the first
    // keyframe's value is null (falls back to neutral value).
    clock.pulse(TimeSpan::ZERO);

    // The animation should continue running (using neutral value as fallback)
    clock.pulse(seconds(0.5));
    assert!(!animation_task.is_completed());

    // Animation completes after its full duration
    clock.pulse(seconds(1.0));
    assert!(animation_task.is_completed());
}

#[test]
fn animation_with_unresolved_binding_does_not_throw() {
    let clock = start();
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(0.0),
        [Setter::with_value(Layoutable::width_property(), unresolved_binding("MissingProperty")) as _],
    ));
    animation.children().add(width_key_frame(300.0, 1.0));

    let control = Border::new();
    control.set_width(50.0);
    let _root = TestRoot::with_child(&control);

    // Start animation - the first keyframe value will be null due to the unresolved binding
    let task = animation.run_async_with_clock(&control, Some(clock.as_clock()), CancellationToken::none());

    // Animation falls back to neutral value and continues
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(seconds(0.1));

    // Animation should still be running (uses neutral value as fallback)
    assert!(!task.is_completed());

    // Animation completes after its full duration
    clock.pulse(seconds(1.0));
    assert!(task.is_completed());
}

fn visibility_animation(from: bool, to: bool) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(0.3));
    animation.set_fill_mode(FillMode::Forward);
    animation.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(Visual::is_visible_property(), from) as _]));
    animation.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Visual::is_visible_property(), to) as _]));
    animation
}

#[test]
fn animation_can_set_is_visible_true_on_invisible_control() {
    let animation = visibility_animation(true, true);

    // Control starts invisible (collapsed state).
    let border = Border::new();
    border.set_is_visible(false);

    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    // Kick off the animation.
    clock.step(TimeSpan::ZERO);

    // The Cue 0.0 keyframe should have set IsVisible = true,
    // even though the control started invisible.
    assert!(border.is_visible());

    // Animation should progress to completion.
    clock.step(seconds(0.3));
    assert!(animation_run.is_completed());
}

#[test]
fn width_animation_resumes_after_is_visible_set_true_on_invisible_control() {
    let animation = Animation::new();
    animation.set_duration(seconds(0.3));
    animation.set_easing(LinearEasing::new());
    animation.set_fill_mode(FillMode::Forward);
    animation.set_playback_behavior(PlaybackBehavior::OnlyIfVisible);
    animation.children().add(width_key_frame(0.0, 0.0));
    animation.children().add(width_key_frame(100.0, 1.0));

    // Control starts invisible (collapsed state).
    let border = Border::new();
    border.set_width(0.0);
    border.set_is_visible(false);

    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    // Animation is paused because control is invisible.
    clock.step(TimeSpan::ZERO);
    assert_eq!(border.width(), 0.0);
    assert!(!animation_run.is_completed());

    // Simulate what the expand handler does: set IsVisible = true externally.
    border.set_is_visible(true);

    // The animation should now resume and complete.
    clock.step(seconds(0.3));
    assert!(animation_run.is_completed());
    assert_eq!(border.width(), 100.0);
}

#[test]
fn animation_can_set_is_visible_false_at_end_without_pausing_itself() {
    let animation = visibility_animation(true, false);

    // Control starts visible (expanded state).
    let border = Border::new();
    border.set_is_visible(true);

    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert!(border.is_visible());

    // Step to the end: animation sets IsVisible=false.
    clock.step(seconds(0.3));

    // Animation should have completed and the final value should hold.
    assert!(animation_run.is_completed());
    assert!(!border.is_visible());
}

#[test]
fn cancelling_expand_animation_mid_flight_then_collapsing_works() {
    let expand_animation = visibility_animation(true, true);
    let collapse_animation = visibility_animation(true, false);

    let border = Border::new();
    border.set_is_visible(false);

    // Start expand.
    let cts1 = CancellationTokenSource::new();
    let clock1 = TestClock::new();
    let expand_run = expand_animation.run_async_with_clock(&border, Some(clock1.as_clock()), cts1.token());

    clock1.step(TimeSpan::ZERO);
    assert!(border.is_visible());

    // Partially through expand, cancel and start collapse.
    clock1.step(seconds(0.15));
    cts1.cancel();
    assert_eq!(expand_run.result(), Some(Ok(())));

    let cts2 = CancellationTokenSource::new();
    let clock2 = TestClock::new();
    let collapse_run = collapse_animation.run_async_with_clock(&border, Some(clock2.as_clock()), cts2.token());

    clock2.step(TimeSpan::ZERO);
    clock2.step(seconds(0.3));

    assert!(collapse_run.is_completed());
    assert!(!border.is_visible());
}

fn three_second_width_animation() -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(3.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(200.0, 1.0));
    animation
}

#[test]
fn auto_pauses_on_invisible_when_started_from_style() {
    // When started via `apply` (the style path), Auto resolves to OnlyIfVisible.
    // The animation should pause when the control becomes invisible.
    let animation = three_second_width_animation();
    let border = border(50.0);
    let clock = TestClock::new();
    let completed = Rc::new(Cell::new(false));
    let c = completed.clone();

    let disposable = animation.apply(
        &border,
        Some(clock.as_clock()),
        Observable::return_(true),
        Some(Rc::new(move || c.set(true))),
        false,
    );

    clock.step(TimeSpan::ZERO);
    assert_eq!(border.width(), 100.0);

    // Hide the control, animation should pause under Auto.
    border.set_is_visible(false);
    clock.step(seconds(1.5));

    // Width should not have advanced while invisible.
    assert_eq!(border.width(), 100.0);

    // Show the control, animation resumes.
    border.set_is_visible(true);
    clock.step(seconds(4.5));

    assert!(completed.get());
    disposable.dispose();
}

#[test]
fn auto_does_not_pause_on_invisible_when_started_manually() {
    // When started manually, Auto resolves to Always.
    // The animation should NOT pause when the control becomes invisible.
    let animation = three_second_width_animation();
    animation.set_easing(LinearEasing::new());
    animation.set_fill_mode(FillMode::Forward);
    let border = border(50.0);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert_eq!(border.width(), 100.0);

    // Hide the control, animation should keep running under Auto + manual.
    border.set_is_visible(false);

    // Width should advance while invisible (not paused).
    clock.step(seconds(1.5));
    assert_eq!(border.width(), 150.0);
    assert!(!animation_run.is_completed());

    clock.step(seconds(3.0));
    assert!(animation_run.is_completed());
    assert_eq!(border.width(), 200.0);
}

#[test]
fn fill_mode_applies_final_value_when_visual_detached_during_animation() {
    let animation = Animation::new();
    animation.set_duration(seconds(5.0));
    animation.set_iteration_count(IterationCount::new(1));
    animation.set_fill_mode(FillMode::Forward);
    animation.children().add(width_key_frame(100.0, 0.0));
    animation.children().add(width_key_frame(300.0, 1.0));

    let border = border(50.0);
    let root = TestRoot::with_child(&border);
    let clock = TestClock::new();
    let animation_run = run(&animation, &border, &clock);

    clock.step(TimeSpan::ZERO);
    assert_eq!(border.width(), 100.0);

    // Detach from visual tree immediately
    root.remove_child(&border);

    // The final value should be applied
    assert!(animation_run.is_completed());
    assert_eq!(border.width(), 300.0);
}
