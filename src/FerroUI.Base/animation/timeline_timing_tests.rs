//! Tests of the timing of timelines ([`super`]), with the cases of WinUI's `CTimeline`.

use super::*;

fn seconds(value: f64) -> Duration {
    Duration::from_secs_f64(value)
}

const NATURAL: Duration = Duration::from_secs(1);

fn timing(duration: f64) -> TimelineTiming {
    TimelineTiming { duration: TimelineDuration::Time(seconds(duration)), ..TimelineTiming::default() }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn the_defaults_are_those_of_winui() {
    let default = TimelineTiming::default();
    assert_eq!(default.begin_time, Some(Duration::ZERO));
    assert_eq!(default.duration, TimelineDuration::Automatic);
    assert_eq!(default.repeat_behavior, RepeatBehavior::Count(1.0));
    assert_eq!(default.fill_behavior, FillBehavior::HoldEnd);
    assert!(!default.auto_reverse);
    assert_eq!(default.speed_ratio, 1.0);
    // Automatic is the natural duration the caller gives.
    assert_eq!(default.active_duration(seconds(0.25)), Some(seconds(0.25)));
}

#[test]
fn a_timeline_is_not_started_before_its_begin_time_and_active_until_it_expires() {
    let delayed = TimelineTiming { begin_time: Some(seconds(1.0)), ..timing(2.0) };
    assert_eq!(delayed.sample(seconds(0.5), NATURAL).state, ClockState::NotStarted);
    let middle = delayed.sample(seconds(2.0), NATURAL);
    assert_eq!(middle.state, ClockState::Active);
    assert!(close(middle.progress, 0.5));
    assert_eq!(delayed.expiration(NATURAL), Some(seconds(3.0)));
    // A timeline without a begin time never starts.
    let never = TimelineTiming { begin_time: None, ..timing(1.0) };
    assert_eq!(never.sample(seconds(5.0), NATURAL).state, ClockState::NotStarted);
    assert_eq!(never.expiration(NATURAL), None);
}

#[test]
fn after_its_active_period_a_timeline_holds_its_end_or_stops() {
    let hold = timing(1.0);
    let sample = hold.sample(seconds(4.0), NATURAL);
    assert_eq!(sample.state, ClockState::Filling);
    assert_eq!(sample.progress, 1.0);
    let stop = TimelineTiming { fill_behavior: FillBehavior::Stop, ..timing(1.0) };
    assert_eq!(stop.sample(seconds(4.0), NATURAL).state, ClockState::Stopped);
    // Within the tolerance of the expiration (0.5 ms) the timeline has expired.
    assert_eq!(hold.sample(seconds(0.9996), NATURAL).state, ClockState::Filling);
}

#[test]
fn repeat_counts_and_durations_set_the_active_period() {
    let twice = TimelineTiming { repeat_behavior: RepeatBehavior::Count(2.0), ..timing(1.5) };
    assert_eq!(twice.active_duration(NATURAL), Some(seconds(3.0)));
    let sample = twice.sample(seconds(2.25), NATURAL);
    assert_eq!((sample.state, sample.iteration), (ClockState::Active, 1));
    assert!(close(sample.progress, 0.5));
    // A fractional count ends inside an iteration and holds there.
    let partial = TimelineTiming { repeat_behavior: RepeatBehavior::Count(1.5), ..timing(1.0) };
    let end = partial.sample(seconds(10.0), NATURAL);
    assert_eq!(end.state, ClockState::Filling);
    assert!(close(end.progress, 0.5));
    // A repeat duration clips the iterations.
    let clipped = TimelineTiming { repeat_behavior: RepeatBehavior::Duration(seconds(2.5)), ..timing(1.0) };
    assert_eq!(clipped.active_duration(NATURAL), Some(seconds(2.5)));
    // For ever.
    let forever = TimelineTiming { repeat_behavior: RepeatBehavior::Forever, ..timing(1.0) };
    assert_eq!(forever.active_duration(NATURAL), None);
    let sample = forever.sample(seconds(100.25), NATURAL);
    assert_eq!((sample.state, sample.iteration), (ClockState::Active, 100));
    assert!(close(sample.progress, 0.25));
    // A duration for ever lasts for ever unless a repeat duration clips it.
    let endless = TimelineTiming { duration: TimelineDuration::Forever, ..TimelineTiming::default() };
    assert_eq!(endless.active_duration(NATURAL), None);
    assert_eq!(endless.sample(seconds(3.0), NATURAL).progress, 0.0);
    let limited = TimelineTiming { repeat_behavior: RepeatBehavior::Duration(seconds(2.0)), ..endless };
    assert_eq!(limited.active_duration(NATURAL), Some(seconds(2.0)));
    // A zero count or duration is a zero active period, which holds the end.
    let zero = TimelineTiming { repeat_behavior: RepeatBehavior::Count(0.0), ..timing(1.0) };
    assert_eq!(zero.active_duration(NATURAL), Some(Duration::ZERO));
    let instant = timing(0.0);
    let sample = instant.sample(Duration::ZERO, NATURAL);
    assert_eq!((sample.state, sample.progress), (ClockState::Filling, 1.0));
}

#[test]
fn auto_reverse_doubles_the_period_and_comes_back() {
    let back = TimelineTiming { auto_reverse: true, ..timing(1.0) };
    assert_eq!(back.active_duration(NATURAL), Some(seconds(2.0)));
    let forward = back.sample(seconds(0.25), NATURAL);
    assert!(close(forward.progress, 0.25));
    let reverse = back.sample(seconds(1.25), NATURAL);
    assert_eq!(reverse.iteration, 1);
    assert!(close(reverse.progress, 0.75));
    // At the end it holds the start.
    let end = back.sample(seconds(3.0), NATURAL);
    assert_eq!(end.state, ClockState::Filling);
    assert!(close(end.progress, 0.0));
}

#[test]
fn the_speed_ratio_scales_the_time_and_a_ratio_that_is_not_positive_is_one() {
    let fast = TimelineTiming { speed_ratio: 2.0, ..timing(1.0) };
    assert_eq!(fast.active_duration(NATURAL), Some(seconds(0.5)));
    assert!(close(fast.sample(seconds(0.25), NATURAL).progress, 0.5));
    let coerced = TimelineTiming { speed_ratio: -3.0, ..timing(1.0) };
    assert_eq!(coerced.active_duration(NATURAL), Some(seconds(1.0)));
}

#[test]
fn from_to_and_by_choose_the_end_points() {
    assert_eq!(from_to_by(Some(1.0), Some(5.0), None, 0.0, 0.0), (1.0, 5.0));
    assert_eq!(from_to_by(None, Some(5.0), None, 2.0, 0.0), (2.0, 5.0));
    assert_eq!(from_to_by(Some(1.0), None, Some(3.0), 0.0, 0.0), (1.0, 4.0));
    assert_eq!(from_to_by(None, None, Some(3.0), 2.0, 0.0), (2.0, 5.0));
    // From alone goes back to the value without animations; To wins over By.
    assert_eq!(from_to_by(Some(1.0), None, None, 2.0, 7.0), (1.0, 7.0));
    assert_eq!(from_to_by(None, Some(5.0), Some(3.0), 2.0, 0.0), (2.0, 5.0));
    assert_eq!(interpolate(2.0, 6.0, 0.25), 3.0);
}

/// The values of the frames of the key frame tests, by index; `None` is the base value.
fn value_at(frames: &[(f64, KeyFrameEase)], values: &[f64], base: f64, progress: f64) -> f64 {
    let segment = key_frame_segment(frames, progress).expect("frames");
    let from = segment.from.map_or(base, |index| values[index]);
    interpolate(from, values[segment.to], segment.progress)
}

#[test]
fn a_linear_key_frame_starts_from_the_base_value_unless_a_frame_is_at_zero() {
    let frames = [(0.5, KeyFrameEase::Linear), (1.0, KeyFrameEase::Linear)];
    let values = [10.0, 20.0];
    assert!(close(value_at(&frames, &values, 0.0, 0.25), 5.0));
    assert!(close(value_at(&frames, &values, 0.0, 0.5), 10.0));
    assert!(close(value_at(&frames, &values, 0.0, 0.75), 15.0));
    assert!(close(value_at(&frames, &values, 0.0, 1.0), 20.0));
    let at_zero = [(0.0, KeyFrameEase::Linear), (1.0, KeyFrameEase::Linear)];
    assert!(close(value_at(&at_zero, &[4.0, 8.0], 100.0, 0.0), 4.0));
    assert!(close(value_at(&at_zero, &[4.0, 8.0], 100.0, 0.5), 6.0));
    assert!(key_frame_segment(&[], 0.5).is_none());
}

#[test]
fn a_discrete_key_frame_holds_the_value_before_it_until_its_key_time() {
    let frames = [(0.0, KeyFrameEase::Discrete), (0.5, KeyFrameEase::Discrete), (0.8, KeyFrameEase::Discrete)];
    let values = [1.0, 2.0, 3.0];
    assert_eq!(value_at(&frames, &values, 0.0, 0.1), 1.0);
    assert_eq!(value_at(&frames, &values, 0.0, 0.49), 1.0);
    assert_eq!(value_at(&frames, &values, 0.0, 0.5), 2.0);
    assert_eq!(value_at(&frames, &values, 0.0, 0.9), 3.0);
    // After the last frame its value is held.
    assert_eq!(value_at(&frames, &values, 0.0, 1.0), 3.0);
    // A first discrete frame after zero holds the base value until it.
    let late = [(0.5, KeyFrameEase::Discrete)];
    assert_eq!(value_at(&late, &[9.0], 4.0, 0.25), 4.0);
    assert_eq!(value_at(&late, &[9.0], 4.0, 0.75), 9.0);
}

#[test]
fn a_spline_key_frame_eases_its_segment() {
    let spline = KeySpline::with_points(0.0, 1.0, 0.0, 1.0);
    let frames = [(1.0, KeyFrameEase::Spline(spline.clone()))];
    let eased = value_at(&frames, &[100.0], 0.0, 0.5);
    assert!(close(eased, 100.0 * spline.get_spline_progress(0.5)), "{eased}");
    assert!(eased > 50.0, "{eased}");
}
