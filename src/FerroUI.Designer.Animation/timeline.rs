use crate::{require, AnimationError, AnimationValue, Easing, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackDirection {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}
/// Native visibility policy. Explicit timeline seeks are independent of visibility;
/// a stateful player uses this policy when advancing its clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackBehavior {
    #[default]
    Auto,
    Always,
    OnlyIfVisible,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    Count(u64),
    Infinite,
}
impl Default for Repeat {
    fn default() -> Self {
        Self::Count(1)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillMode {
    None,
    #[default]
    Forward,
    Backward,
    Both,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f64,
    pub value: AnimationValue,
    /// Eases the incoming segment, as FerroUI's KeyFrame.KeySpline does.
    pub easing: Easing,
}
impl Keyframe {
    pub fn new(time: f64, value: AnimationValue) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub target: u64,
    pub property: String,
    pub keyframes: Vec<Keyframe>,
    /// The value before animation. Required when importing a runtime track with implicit
    /// endpoint values when FillMode does not hold that endpoint. Authored tracks
    /// without it hold their closest explicit keyframe.
    pub base_value: Option<AnimationValue>,
    pub muted: bool,
}
impl Track {
    pub fn new(target: u64, property: impl Into<String>) -> Self {
        Self {
            target,
            property: property.into(),
            keyframes: vec![],
            base_value: None,
            muted: false,
        }
    }
    pub fn record(&mut self, time: f64, value: AnimationValue) -> Result<()> {
        require(
            time.is_finite() && time >= 0.0,
            "Keyframe time must be finite and nonnegative",
        )?;
        let time = if time == 0.0 { 0.0 } else { time };
        value.validate()?;
        if let Some(base) = &self.base_value {
            require(
                base.kind() == value.kind(),
                "Recorded value differs from the underlying property type",
            )?;
        }
        if let Some(existing) = self.keyframes.first() {
            require(
                existing.value.kind() == value.kind(),
                "All values in a track must have the same type",
            )?;
        }
        match self.keyframes.binary_search_by(|k| k.time.total_cmp(&time)) {
            Ok(index) => self.keyframes[index].value = value,
            Err(index) => {
                require(self.keyframes.len() < 100_000, "Too many keyframes")?;
                self.keyframes.insert(index, Keyframe::new(time, value));
            }
        }
        Ok(())
    }
    pub fn remove_keyframe(&mut self, time: f64) -> Result<Keyframe> {
        let time = if time == 0.0 { 0.0 } else { time };
        let index = self
            .keyframes
            .binary_search_by(|k| k.time.total_cmp(&time))
            .map_err(|_| AnimationError("Keyframe not found".into()))?;
        Ok(self.keyframes.remove(index))
    }
    pub fn move_keyframe(&mut self, from: f64, to: f64) -> Result<()> {
        let from = if from == 0.0 { 0.0 } else { from };
        let to = if to == 0.0 { 0.0 } else { to };
        require(
            to.is_finite() && to >= 0.0,
            "Keyframe time must be finite and nonnegative",
        )?;
        if from == to {
            return self
                .keyframes
                .iter()
                .any(|k| k.time == from)
                .then_some(())
                .ok_or_else(|| AnimationError("Keyframe not found".into()));
        }
        require(
            !self.keyframes.iter().any(|k| k.time == to),
            "Another keyframe occupies the destination time",
        )?;
        let mut keyframe = self.remove_keyframe(from)?;
        keyframe.time = to;
        let index = self.keyframes.partition_point(|k| k.time < to);
        self.keyframes.insert(index, keyframe);
        Ok(())
    }
    pub fn set_easing(&mut self, time: f64, easing: Easing) -> Result<()> {
        easing.validate()?;
        require(
            !matches!(easing, Easing::Named(_)),
            "Per-keyframe easing must be Linear or CubicBezier; named easing belongs to the timeline",
        )?;
        let key = self
            .keyframes
            .iter_mut()
            .find(|k| k.time == time)
            .ok_or_else(|| AnimationError("Keyframe not found".into()))?;
        key.easing = easing;
        Ok(())
    }
    pub fn validate(&self, duration: f64) -> Result<()> {
        require(self.target > 0, "Animation target must be nonzero")?;
        require(
            !self.property.is_empty()
                && self.property.len() <= 1024
                && !self
                    .property
                    .chars()
                    .any(|ch| ch.is_control() || matches!(ch, '\u{FFFE}' | '\u{FFFF}')),
            "Invalid animation property",
        )?;
        require(self.keyframes.len() <= 100_000, "Too many keyframes")?;
        if let Some(value) = &self.base_value {
            value.validate()?;
        }
        let mut previous = None;
        let mut kind = self.base_value.as_ref().map(AnimationValue::kind);
        for key in &self.keyframes {
            require(
                key.time.is_finite() && key.time >= 0.0 && key.time <= duration,
                "Keyframe must lie within the timeline",
            )?;
            if let Some(time) = previous {
                require(time < key.time, "Keyframes must be strictly increasing")?;
            }
            previous = Some(key.time);
            key.value.validate()?;
            key.easing.validate()?;
            require(
                !matches!(key.easing, Easing::Named(_)),
                "Only linear or cubic Bezier easing is valid on a keyframe",
            )?;
            if let Some(existing) = kind {
                require(
                    existing == key.value.kind(),
                    "Track value types do not match",
                )?;
            }
            kind = Some(key.value.kind());
        }
        Ok(())
    }
    fn sample_validated(
        &self,
        time: f64,
        duration: f64,
        fill: FillMode,
    ) -> Result<Option<AnimationValue>> {
        if self.muted || self.keyframes.is_empty() {
            return Ok(None);
        }
        let first = &self.keyframes[0];
        let last = self.keyframes.last().expect("nonempty track");
        let before = if matches!(fill, FillMode::Backward | FillMode::Both) {
            &first.value
        } else {
            self.base_value.as_ref().unwrap_or(&first.value)
        };
        let after = if matches!(fill, FillMode::Forward | FillMode::Both) {
            &last.value
        } else {
            self.base_value.as_ref().unwrap_or(&last.value)
        };
        let index = self.keyframes.partition_point(|k| k.time < time);
        if index < self.keyframes.len() && self.keyframes[index].time == time {
            return Ok(Some(self.keyframes[index].value.clone()));
        }
        let (start, end, start_time, end_time, easing) = if index == 0 {
            if first.time == 0.0 {
                if self.keyframes.len() == 1 {
                    return Ok(Some(first.value.clone()));
                }
                let next = &self.keyframes[1];
                (
                    &first.value,
                    &next.value,
                    first.time,
                    next.time,
                    &next.easing,
                )
            } else {
                (before, &first.value, 0.0, first.time, &first.easing)
            }
        } else if index == self.keyframes.len() {
            if last.time == duration {
                if self.keyframes.len() == 1 {
                    return Ok(Some(last.value.clone()));
                }
                let prev = &self.keyframes[self.keyframes.len() - 2];
                (&prev.value, &last.value, prev.time, last.time, &last.easing)
            } else {
                (&last.value, after, last.time, duration, &last.easing)
            }
        } else {
            let prev = &self.keyframes[index - 1];
            let next = &self.keyframes[index];
            (&prev.value, &next.value, prev.time, next.time, &next.easing)
        };
        let progress = (time - start_time) / (end_time - start_time);
        let progress = if matches!(easing, Easing::Linear) {
            progress
        } else {
            easing.sample(progress)?
        };
        Ok(Some(start.interpolate(end, progress)?))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub duration: f64,
    pub delay: f64,
    #[serde(default)]
    pub delay_between_iterations: f64,
    #[serde(default)]
    pub playback_behavior: PlaybackBehavior,
    pub speed_ratio: f64,
    pub repeat: Repeat,
    pub playback_direction: PlaybackDirection,
    pub fill: FillMode,
    /// Global progress easing, applied before selecting an incoming keyframe segment.
    pub easing: Easing,
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub target: u64,
    pub property: String,
    pub value: AnimationValue,
}
impl Timeline {
    pub fn new(duration: f64) -> Result<Self> {
        let result = Self {
            duration,
            delay: 0.0,
            delay_between_iterations: 0.0,
            playback_behavior: PlaybackBehavior::Auto,
            speed_ratio: 1.0,
            repeat: Repeat::default(),
            playback_direction: PlaybackDirection::Normal,
            fill: FillMode::Forward,
            easing: Easing::Linear,
            tracks: vec![],
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        require(
            self.duration.is_finite() && self.duration > 0.0,
            "Timeline duration must be finite and positive",
        )?;
        require(
            self.delay.is_finite() && self.delay >= 0.0,
            "Timeline delay must be finite and nonnegative",
        )?;
        require(
            self.delay_between_iterations.is_finite()
                && self.delay_between_iterations >= 0.0
                && (self.duration + self.delay_between_iterations).is_finite(),
            "Delay between iterations must be finite, nonnegative and fit the clock range",
        )?;
        require(
            self.speed_ratio.is_finite() && self.speed_ratio > 0.0,
            "Playback speed must be finite and positive",
        )?;
        if let Repeat::Count(count) = self.repeat {
            require(
                count > 0
                    && count <= i64::MAX as u64
                    && ((self.duration * count as f64
                        + self.delay_between_iterations * (count - 1) as f64
                        + self.delay)
                        / self.speed_ratio)
                        .is_finite(),
                "Repeat count must fit native Int64 with a finite total playback duration",
            )?;
        }
        require(
            (self.duration / self.speed_ratio).is_finite()
                && self.duration / self.speed_ratio > 0.0
                && (self.delay / self.speed_ratio).is_finite(),
            "Effective playback timing is outside the finite clock range",
        )?;
        self.easing.validate()?;
        require(self.tracks.len() <= 4096, "Too many animation tracks")?;
        let mut keys = BTreeSet::new();
        for track in &self.tracks {
            require(
                keys.insert((track.target, &track.property)),
                "Duplicate target/property animation track",
            )?;
            track.validate(self.duration)?;
        }
        Ok(())
    }
    pub fn record(
        &mut self,
        target: u64,
        property: impl Into<String>,
        time: f64,
        value: AnimationValue,
    ) -> Result<()> {
        require(
            time.is_finite() && time >= 0.0 && time <= self.duration,
            "Recording time must be within the timeline",
        )?;
        let property = property.into();
        if let Some(track) = self
            .tracks
            .iter_mut()
            .find(|track| track.target == target && track.property == property)
        {
            return track.record(time, value);
        }
        let mut track = Track::new(target, property);
        track.record(time, value)?;
        track.validate(self.duration)?;
        require(self.tracks.len() < 4096, "Too many animation tracks")?;
        self.tracks.push(track);
        Ok(())
    }
    pub fn track(&self, target: u64, property: &str) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|t| t.target == target && t.property == property)
    }
    pub fn track_mut(&mut self, target: u64, property: &str) -> Option<&mut Track> {
        self.tracks
            .iter_mut()
            .find(|t| t.target == target && t.property == property)
    }
    pub fn remove_track(&mut self, target: u64, property: &str) -> bool {
        let previous = self.tracks.len();
        self.tracks
            .retain(|t| t.target != target || t.property != property);
        previous != self.tracks.len()
    }
    /// Compiles once after authoring, then samples in O(tracks * log(keyframes)) per frame.
    pub fn compile(&self) -> Result<CompiledTimeline> {
        self.validate()?;
        Ok(CompiledTimeline(self.clone()))
    }
    pub fn sample(&self, time: f64) -> Result<Vec<Sample>> {
        self.validate()?;
        self.sample_validated(time)
    }
    /// Samples a single iteration directly for timeline scrubbing, ignoring delay/repeat/direction.
    pub fn scrub(&self, time: f64) -> Result<Vec<Sample>> {
        self.validate()?;
        require(time.is_finite(), "Scrub time must be finite")?;
        self.sample_tracks(time.clamp(0.0, self.duration))
    }
    /// Whether the running clock pauses for an effectively invisible target.
    pub fn pauses_when_hidden(&self, manually_started: bool) -> bool {
        match self.playback_behavior {
            PlaybackBehavior::Always => false,
            PlaybackBehavior::OnlyIfVisible => true,
            PlaybackBehavior::Auto => {
                !manually_started
                    && !self
                        .tracks
                        .iter()
                        .any(|track| !track.muted && track.property == "IsVisible")
            }
        }
    }
    pub fn total_duration(&self) -> Option<f64> {
        match self.repeat {
            Repeat::Count(count) => Some(
                (self.delay
                    + self.duration * count as f64
                    + self.delay_between_iterations * (count - 1) as f64)
                    / self.speed_ratio,
            ),
            Repeat::Infinite => None,
        }
    }
    fn sample_validated(&self, time: f64) -> Result<Vec<Sample>> {
        require(
            time.is_finite() && time >= 0.0,
            "Playback time must be finite and nonnegative",
        )?;
        let elapsed = time * self.speed_ratio - self.delay;
        require(elapsed.is_finite(), "Playback time overflow")?;
        if elapsed < 0.0 || (self.delay > 0.0 && elapsed == 0.0) {
            if !matches!(self.fill, FillMode::Backward | FillMode::Both) {
                return Ok(vec![]);
            }
            // Native initial-delay fill uses the first actual keyframe in playback
            // direction, without applying global easing or an implicit endpoint.
            let reverse = matches!(
                self.playback_direction,
                PlaybackDirection::Reverse | PlaybackDirection::AlternateReverse
            );
            return Ok(self
                .tracks
                .iter()
                .filter(|track| !track.muted)
                .filter_map(|track| {
                    let key = if reverse {
                        track.keyframes.last()
                    } else {
                        track.keyframes.first()
                    }?;
                    Some(Sample {
                        target: track.target,
                        property: track.property.clone(),
                        value: key.value.clone(),
                    })
                })
                .collect());
        }
        let cycle = self.duration + self.delay_between_iterations;
        let (iteration, progress) = if let Repeat::Count(count) = self.repeat {
            let end =
                self.duration * count as f64 + self.delay_between_iterations * (count - 1) as f64;
            if elapsed >= end {
                if !matches!(self.fill, FillMode::Forward | FillMode::Both) {
                    return Ok(vec![]);
                }
                (count - 1, 1.0)
            } else {
                (
                    (elapsed / cycle).floor() as u64,
                    ((elapsed % cycle) / self.duration).min(1.0),
                )
            }
        } else {
            let iteration = (elapsed / cycle).floor();
            require(
                iteration.is_finite() && iteration < u64::MAX as f64,
                "Playback iteration exceeds the clock index range",
            )?;
            (
                iteration as u64,
                ((elapsed % cycle) / self.duration).min(1.0),
            )
        };
        let reverse = match self.playback_direction {
            PlaybackDirection::Normal => false,
            PlaybackDirection::Reverse => true,
            PlaybackDirection::Alternate => iteration % 2 == 1,
            PlaybackDirection::AlternateReverse => iteration % 2 == 0,
        };
        let progress = if reverse { 1.0 - progress } else { progress };
        self.sample_tracks(self.easing.sample(progress)? * self.duration)
    }
    fn sample_tracks(&self, time: f64) -> Result<Vec<Sample>> {
        let mut result = Vec::with_capacity(self.tracks.len());
        for track in &self.tracks {
            if let Some(value) = track.sample_validated(time, self.duration, self.fill)? {
                result.push(Sample {
                    target: track.target,
                    property: track.property.clone(),
                    value,
                });
            }
        }
        Ok(result)
    }
}
/// Immutable validated timeline for the render loop. Recompile after an authored edit.
#[derive(Clone, Debug)]
pub struct CompiledTimeline(Timeline);
impl CompiledTimeline {
    pub fn sample(&self, time: f64) -> Result<Vec<Sample>> {
        self.0.sample_validated(time)
    }
    /// Scrub the validated edit timeline without rescanning keyframes. As with
    /// Timeline::scrub, delay, repeat and playback direction are intentionally ignored.
    pub fn scrub(&self, time: f64) -> Result<Vec<Sample>> {
        require(time.is_finite(), "Scrub time must be finite")?;
        self.0.sample_tracks(time.clamp(0.0, self.0.duration))
    }
    pub fn duration(&self) -> f64 {
        self.0.duration
    }
    pub fn total_duration(&self) -> Option<f64> {
        self.0.total_duration()
    }
}

/// Clock-free playback controller. The host supplies measured elapsed time, making seeks and
/// tests independent of frame rate. No timer/thread is created by this crate.
#[derive(Clone, Debug, Default)]
pub struct Player {
    time: f64,
    playing: bool,
}
impl Player {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    pub fn play(&mut self) {
        self.playing = true;
    }
    pub fn pause(&mut self) {
        self.playing = false;
    }
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
    }
    pub fn seek(&mut self, time: f64) -> Result<()> {
        require(
            time.is_finite() && time >= 0.0,
            "Seek time must be finite and nonnegative",
        )?;
        self.time = time;
        Ok(())
    }
    /// Advance without charging hidden time to policies that pause when invisible.
    /// This preserves the elapsed position and resumes without a catch-up jump.
    pub fn advance_with_visibility(
        &mut self,
        delta: f64,
        timeline: &Timeline,
        visible: bool,
        manually_started: bool,
    ) -> Result<Vec<Sample>> {
        require(
            delta.is_finite() && delta >= 0.0,
            "Clock delta must be finite and nonnegative",
        )?;
        self.advance(
            if !visible && timeline.pauses_when_hidden(manually_started) {
                0.0
            } else {
                delta
            },
            timeline,
        )
    }
    pub fn advance(&mut self, delta: f64, timeline: &Timeline) -> Result<Vec<Sample>> {
        require(
            delta.is_finite() && delta >= 0.0,
            "Clock delta must be finite and nonnegative",
        )?;
        timeline.validate()?;
        let mut time = self.time;
        let mut playing = self.playing;
        if playing {
            time += delta;
            require(time.is_finite(), "Playback clock overflow")?;
            if let Some(end) = timeline.total_duration() {
                if time >= end {
                    time = end;
                    playing = false;
                }
            }
        }
        let result = timeline.sample_validated(time)?;
        self.time = time;
        self.playing = playing;
        Ok(result)
    }
}
