use crate::{AnimationError, Result, Timeline, native_key_time, require};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyframeLocation {
    pub target: u64,
    pub property: String,
    pub time: f64,
}

impl KeyframeLocation {
    /// Follow the source time accepted by a codec after a precision-limited edit.
    pub fn reconcile_time(&mut self, timeline: &Timeline) {
        let Some(track) = timeline.track(self.target, &self.property) else {
            return;
        };
        let index = track
            .keyframes
            .partition_point(|candidate| candidate.time < self.time);
        let nearest = [index.checked_sub(1), Some(index)]
            .into_iter()
            .flatten()
            .filter_map(|index| track.keyframes.get(index))
            .min_by(|left, right| {
                (left.time - self.time)
                    .abs()
                    .total_cmp(&(right.time - self.time).abs())
            });
        if let Some(key) = nearest.filter(|key| (key.time - self.time).abs() <= 0.0000001) {
            self.time = key.time;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyframeGroupEdit {
    Move(f64),
    Scale { pivot: f64, factor: f64 },
    Duplicate(f64),
    Remove,
}

impl KeyframeGroupEdit {
    pub fn retimes(self) -> bool {
        matches!(self, Self::Move(_) | Self::Scale { .. })
    }

    fn validate(self, duration: f64) -> Result<()> {
        match self {
            Self::Move(offset) | Self::Duplicate(offset) => {
                require(offset.is_finite(), "The keyframe offset must be finite")
            }
            Self::Scale { pivot, factor } => {
                require(
                    (0.0..=duration).contains(&pivot),
                    "Choose a scale pivot within the timeline",
                )?;
                require(
                    factor.is_finite() && factor > 0.0,
                    "The keyframe scale must be positive and finite",
                )
            }
            Self::Remove => Ok(()),
        }
    }

    fn destination(self, time: f64, duration: f64) -> Result<f64> {
        let destination = normalized_time(match self {
            Self::Move(offset) | Self::Duplicate(offset) => time + offset,
            Self::Scale { pivot, factor } if factor != 1.0 => pivot + (time - pivot) * factor,
            _ => time,
        });
        require(
            (0.0..=duration).contains(&destination),
            "The selected keyframes would leave the animation duration",
        )?;
        if matches!(self, Self::Scale { .. }) && destination != time {
            // Scaled keys can land within one native TimeSpan tick of each other.
            // Check collisions at the precision that both source encodings accept.
            native_key_time(destination)
        } else {
            Ok(destination)
        }
    }
}

impl Timeline {
    /// Validate a group edit against the current tracks and return a single replacement.
    /// Return destinations in input order so the host can keep its ordered selection.
    pub fn with_keyframe_edit(
        &self,
        keys: &[KeyframeLocation],
        edit: KeyframeGroupEdit,
    ) -> Result<(Self, Vec<KeyframeLocation>)> {
        require(!keys.is_empty(), "Select one or more keyframes")?;
        edit.validate(self.duration)?;
        let mut groups = BTreeMap::<(u64, &str), BTreeMap<u64, f64>>::new();
        let mut changed = Vec::new();
        let remove = matches!(edit, KeyframeGroupEdit::Remove);
        for key in keys {
            require(
                key.time.is_finite() && (0.0..=self.duration).contains(&key.time),
                "A selected keyframe lies outside the timeline",
            )?;
            let time = normalized_time(key.time);
            let destination = edit.destination(time, self.duration)?;
            let unique = groups
                .entry((key.target, &key.property))
                .or_default()
                .insert(time.to_bits(), destination)
                .is_none();
            require(unique, "Select each keyframe only once")?;
            if !remove {
                changed.push(KeyframeLocation {
                    time: destination,
                    ..key.clone()
                });
            }
        }
        let mut timeline = self.clone();
        for ((target, property), times) in groups {
            let track = timeline
                .track_mut(target, property)
                .ok_or_else(|| AnimationError("Animation track no longer exists".into()))?;
            let moving = track
                .keyframes
                .iter()
                .filter_map(|key| {
                    let time = times.get(&normalized_time(key.time).to_bits())?;
                    let mut key = key.clone();
                    key.time = *time;
                    Some(key)
                })
                .collect::<Vec<_>>();
            require(
                moving.len() == times.len(),
                "A selected keyframe no longer exists",
            )?;
            // Remove the full selection before inserting its destinations. Shifts
            // can then cross old selected times without reporting a false collision.
            if !matches!(edit, KeyframeGroupEdit::Duplicate(_)) {
                track
                    .keyframes
                    .retain(|key| !times.contains_key(&normalized_time(key.time).to_bits()));
            }
            if !remove {
                if matches!(edit, KeyframeGroupEdit::Scale { .. }) {
                    require(
                        moving
                            .windows(2)
                            .all(|keys| keys[1].time - keys[0].time > 1e-9),
                        "Selected keyframes would share a destination time; no keys were changed",
                    )?;
                }
                let stationary = std::mem::take(&mut track.keyframes);
                let mut merged = Vec::with_capacity(stationary.len() + moving.len());
                let mut left = stationary.into_iter().peekable();
                let mut right = moving.into_iter().peekable();
                while let (Some(existing), Some(incoming)) = (left.peek(), right.peek()) {
                    require(
                        (existing.time - incoming.time).abs() > 1e-9,
                        "A keyframe already occupies a destination time; no keys were changed",
                    )?;
                    if existing.time < incoming.time {
                        merged.push(left.next().expect("peeked key"));
                    } else {
                        merged.push(right.next().expect("peeked key"));
                    }
                }
                merged.extend(left);
                merged.extend(right);
                track.keyframes = merged;
            }
        }
        timeline.tracks.retain(|track| !track.keyframes.is_empty());
        timeline.validate()?;
        Ok((timeline, changed))
    }
}

fn normalized_time(time: f64) -> f64 {
    if time == 0.0 { 0.0 } else { time }
}
