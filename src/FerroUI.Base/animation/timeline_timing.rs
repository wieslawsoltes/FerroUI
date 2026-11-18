//! The timing of WinUI's timelines (#443, VSM-3 of docs/porting/winui-calculator-gaps.md,
//! section 5.8): when a timeline is active, the progress of its current iteration, its fill, and
//! the segment of a key frame animation a progress falls in.
//!
//! Port of the clock state computation of WinUI's `CTimeline`
//! (`dxaml/xcp/core/animation/Timeline.cpp`: `ComputeEffectiveDuration`,
//! `ComputeExpirationTime`, `ComputeCurrentClockState`, `ComputeLocalProgressAndTime`,
//! `IsExpired`) and of the segment search of `CAnimation` (`dxaml/xcp/core/animation/animation.cpp`,
//! the key frame part of `UpdateAnimation`) of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0` (MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md). The path of dependent animations: FerroUI animates every property on the UI thread,
//! so the waits for composition animations of the source do not exist here.
//!
//! These are values and functions, not objects: the storyboard family of `ferroui-controls`
//! (`Storyboard`, `DoubleAnimation`, the key frame animations) holds its properties and asks this
//! module, with the time of its clock, what to write.

use std::rc::Rc;
use std::time::Duration;

use super::easings::IEasing;
use super::KeySpline;
use crate::Ref;

/// The tolerance of the expiration test, as WinUI's `s_timeTolerance` (0.5 milliseconds).
const TIME_TOLERANCE: f64 = 0.0005;

/// The duration of a timeline (WinUI's `Duration`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimelineDuration {
    /// The natural duration of the timeline: one second for an animation from, to or by a value,
    /// the last key time for a key frame animation, the end of the last child for a storyboard.
    Automatic,
    /// Never ends.
    Forever,
    /// A time.
    Time(Duration),
}

/// How a timeline repeats (WinUI's `RepeatBehavior`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatBehavior {
    /// A number of iterations (`1x`, `2.5x`).
    Count(f64),
    /// Repeats until a time has passed.
    Duration(Duration),
    /// Repeats for ever.
    Forever,
}

/// What a timeline does after its active period (WinUI's `FillBehavior`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillBehavior {
    /// Holds the value of the end of the active period.
    #[default]
    HoldEnd,
    /// Stops: the animated value is removed.
    Stop,
}

/// The state of the clock of a timeline (WinUI's `ClockState`, with the time before the begin
/// time as its own state).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockState {
    /// Before the begin time: the timeline has no effect yet.
    NotStarted,
    /// In the active period.
    Active,
    /// After the active period, holding the end ([`FillBehavior::HoldEnd`]).
    Filling,
    /// After the active period, without effect ([`FillBehavior::Stop`]).
    Stopped,
}

/// The timing properties of a timeline (WinUI's `Timeline`: `BeginTime`, `Duration`,
/// `AutoReverse`, `RepeatBehavior`, `SpeedRatio`, `FillBehavior`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineTiming {
    /// When the timeline starts, relative to its parent; `None` never starts.
    pub begin_time: Option<Duration>,
    pub duration: TimelineDuration,
    pub auto_reverse: bool,
    pub repeat_behavior: RepeatBehavior,
    /// The rate of the time of the timeline relative to its parent; a ratio that is not positive
    /// is 1, as WinUI coerces it.
    pub speed_ratio: f64,
    pub fill_behavior: FillBehavior,
}

impl Default for TimelineTiming {
    fn default() -> Self {
        Self {
            begin_time: Some(Duration::ZERO),
            duration: TimelineDuration::Automatic,
            auto_reverse: false,
            repeat_behavior: RepeatBehavior::Count(1.0),
            speed_ratio: 1.0,
            fill_behavior: FillBehavior::HoldEnd,
        }
    }
}

/// The state of a timeline at a time of its parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineSample {
    pub state: ClockState,
    /// The progress of the current iteration, 0 to 1, reversed in the reverse half of an
    /// iteration of an auto-reversing timeline; 0 before the start and for a timeline that lasts
    /// for ever.
    pub progress: f64,
    /// The index of the current iteration (each half of an auto-reversing timeline counts).
    pub iteration: u64,
}

impl TimelineTiming {
    fn speed(&self) -> f64 {
        if self.speed_ratio > 0.0 && self.speed_ratio.is_finite() {
            self.speed_ratio
        } else {
            1.0
        }
    }

    /// The duration with `natural` for [`TimelineDuration::Automatic`]: `None` for ever.
    fn resolved(&self, natural: Duration) -> Option<f64> {
        match self.duration {
            TimelineDuration::Automatic => Some(natural.as_secs_f64()),
            TimelineDuration::Forever => None,
            TimelineDuration::Time(time) => Some(time.as_secs_f64()),
        }
    }

    /// The length of the active period, in the time of the parent (WinUI's
    /// `ComputeEffectiveDuration`): `None` for ever. `natural` is the duration of
    /// [`TimelineDuration::Automatic`].
    pub fn active_duration(&self, natural: Duration) -> Option<Duration> {
        self.effective(natural).map(Duration::from_secs_f64)
    }

    fn effective(&self, natural: Duration) -> Option<f64> {
        let speed = self.speed();
        let duration = self.resolved(natural);
        let (count, limit) = match self.repeat_behavior {
            RepeatBehavior::Duration(limit) => (1.0, limit.as_secs_f64()),
            RepeatBehavior::Count(count) => (count, f64::MAX),
            RepeatBehavior::Forever => (1.0, f64::MAX),
        };
        if duration.is_some_and(|duration| duration <= 0.0) || count <= 0.0 {
            return Some(0.0);
        }
        let forever_repeat = matches!(self.repeat_behavior, RepeatBehavior::Forever);
        let Some(duration) = duration else {
            // Lasts for ever: a count does not clip it, a duration of repeat does.
            return match self.repeat_behavior {
                RepeatBehavior::Duration(_) => Some(limit),
                _ => None,
            };
        };
        if forever_repeat {
            return None;
        }
        let mut scaling = match self.repeat_behavior {
            RepeatBehavior::Duration(_) => limit / speed,
            _ => count / speed,
        };
        if self.auto_reverse {
            scaling *= 2.0;
        }
        Some((scaling * duration).min(limit))
    }

    /// When the active period ends, in the time of the parent (WinUI's `ComputeExpirationTime`):
    /// `None` when it never ends or the timeline never starts.
    pub fn expiration(&self, natural: Duration) -> Option<Duration> {
        let begin = self.begin_time?.as_secs_f64();
        self.effective(natural).map(|effective| Duration::from_secs_f64(begin + effective))
    }

    /// The state of the timeline at the time `parent_time` of its parent; `natural` is the
    /// duration of [`TimelineDuration::Automatic`].
    pub fn sample(&self, parent_time: Duration, natural: Duration) -> TimelineSample {
        let not_started = TimelineSample { state: ClockState::NotStarted, progress: 0.0, iteration: 0 };
        let Some(begin) = self.begin_time.map(|begin| begin.as_secs_f64()) else {
            return not_started;
        };
        let mut time = parent_time.as_secs_f64();
        if time < begin {
            return not_started;
        }
        let expiration = self.effective(natural).map(|effective| begin + effective);
        let expired = |time: f64| expiration.is_some_and(|expiration| expiration <= time + TIME_TOLERANCE);
        let state = if expired(time) {
            match self.fill_behavior {
                FillBehavior::HoldEnd => {
                    // Clamped to the expiration time.
                    time = expiration.unwrap_or(time);
                    ClockState::Filling
                }
                FillBehavior::Stop => ClockState::Stopped,
            }
        } else {
            ClockState::Active
        };
        if state == ClockState::Stopped {
            return TimelineSample { state, progress: 0.0, iteration: 0 };
        }
        let (progress, iteration) = self.local_progress(begin, time, natural, expired(time));
        TimelineSample { state, progress, iteration }
    }

    /// WinUI's `ComputeLocalProgressAndTime`: the progress and the iteration at `time` (at least
    /// `begin`), with `expired` whether the active period is over at that time.
    fn local_progress(&self, begin: f64, time: f64, natural: Duration, expired: bool) -> (f64, u64) {
        let current = (time - begin) * self.speed();
        let Some(duration) = self.resolved(natural) else {
            return (0.0, 0);
        };
        if duration <= 0.0 {
            // A zero-length timeline holds its end value.
            return (1.0, 0);
        }
        let iterations = current / duration;
        let iteration = iterations.floor();
        let mut progress = iterations - iteration;
        let iteration = iteration.max(0.0) as u64;
        // At the boundary of an iteration when the active period is over: the end of the last
        // iteration (the start, for an auto-reversing timeline that came back).
        if ((iterations >= 1.0 && progress <= 0.001) || progress >= 0.999) && expired {
            progress = match self.auto_reverse && iteration % 2 == 0 {
                true => 0.0,
                false => 1.0,
            };
        }
        if self.auto_reverse && iteration % 2 == 1 {
            progress = 1.0 - progress;
        }
        (progress, iteration)
    }
}

/// The end points of an animation from, to or by a value (WinUI's `CAnimation::OnBegin` for the
/// animations without key frames): `from` is `From` or the base value; `to` is `To`, or
/// `from + By`, or the value of the property without animations when neither is set.
pub fn from_to_by(from: Option<f64>, to: Option<f64>, by: Option<f64>, base: f64, non_animated_base: f64) -> (f64, f64) {
    let start = from.unwrap_or(base);
    let end = match (to, by) {
        (Some(to), _) => to,
        (None, Some(by)) => start + by,
        (None, None) => non_animated_base,
    };
    (start, end)
}

/// How a key frame reaches its value from the value before it (WinUI's `Discrete`, `Linear`,
/// `Spline` and `Easing` key frames).
#[derive(Clone)]
pub enum KeyFrameEase {
    /// Holds the value before it until its key time.
    Discrete,
    Linear,
    Spline(Ref<KeySpline>),
    Easing(Rc<dyn IEasing>),
}

impl KeyFrameEase {
    /// WinUI's `GetEffectiveProgress` of the key frame.
    pub fn effective_progress(&self, progress: f64) -> f64 {
        match self {
            Self::Discrete => 0.0,
            Self::Linear => progress,
            Self::Spline(spline) => spline.get_spline_progress(progress),
            Self::Easing(easing) => easing.ease(progress),
        }
    }
}

/// The value a key frame animation interpolates at a progress: from the value `from` (`None`: the
/// base value) to the key frame `to`, with the progress of the segment between them (already
/// eased by the kind of the key frame).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyFrameSegment {
    pub from: Option<usize>,
    pub to: usize,
    pub progress: f64,
}

/// The segment of the key frames `frames` at the progress `progress` of the animation (WinUI's
/// segment search of `CAnimation`). Each frame is its key time as a fraction of the duration of
/// the animation, in ascending order, and its kind. `None` without frames.
pub fn key_frame_segment(frames: &[(f64, KeyFrameEase)], progress: f64) -> Option<KeyFrameSegment> {
    let count = frames.len();
    let first = frames.first()?;
    // The first segment: from the base value to the first frame, unless that frame is at 0.
    let mut from = None;
    let mut span = first.0;
    if first.0 <= 0.0 {
        from = Some(0);
    }
    let mut to = 0;
    let mut current = 0;
    let mut cumulative = 0.0;
    while current < count && frames[current].0 <= progress {
        current += 1;
        from = Some(to);
        cumulative += span;
        if current < count {
            to = current;
            span = frames[current].0 - cumulative;
        } else {
            // After the last frame: its value is held.
            to = count - 1;
            span = if cumulative < 1.0 { 1.0 - cumulative } else { 1.0 };
        }
    }
    let linear = if span != 0.0 { (progress - cumulative) / span } else { 1.0 };
    let frame = &frames[current.min(count - 1)];
    Some(KeyFrameSegment { from, to, progress: frame.1.effective_progress(linear) })
}

/// The value at a segment: `from` and `to` interpolated linearly with the progress of the segment.
pub fn interpolate(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}

#[cfg(test)]
#[path = "timeline_timing_tests.rs"]
mod tests;
