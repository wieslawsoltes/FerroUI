use crate::{AnimationError, Result, Timeline, require};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    Duplicate(f64),
    Remove,
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
        let delta = match edit {
            KeyframeGroupEdit::Move(delta) | KeyframeGroupEdit::Duplicate(delta) => {
                require(delta.is_finite(), "The keyframe offset must be finite")?;
                Some(delta)
            }
            KeyframeGroupEdit::Remove => None,
        };
        let mut groups = BTreeMap::<(u64, &str), BTreeSet<u64>>::new();
        for key in keys {
            require(
                key.time.is_finite() && (0.0..=self.duration).contains(&key.time),
                "A selected keyframe lies outside the timeline",
            )?;
            let time = normalized_time(key.time);
            let unique = groups
                .entry((key.target, &key.property))
                .or_default()
                .insert(time.to_bits());
            require(unique, "Select each keyframe only once")?;
        }
        let mut timeline = self.clone();
        for ((target, property), times) in groups {
            let track = timeline
                .track_mut(target, property)
                .ok_or_else(|| AnimationError("Animation track no longer exists".into()))?;
            let mut moving = track
                .keyframes
                .iter()
                .filter(|key| times.contains(&normalized_time(key.time).to_bits()))
                .cloned()
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
                    .retain(|key| !times.contains(&normalized_time(key.time).to_bits()));
            }
            if let Some(delta) = delta {
                for key in &mut moving {
                    key.time = normalized_time(key.time + delta);
                    require(
                        key.time.is_finite() && (0.0..=self.duration).contains(&key.time),
                        "The selected keyframes would leave the animation duration",
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
        let changed = delta.map_or_else(Vec::new, |delta| {
            keys.iter()
                .map(|key| KeyframeLocation {
                    time: normalized_time(key.time + delta),
                    ..key.clone()
                })
                .collect()
        });
        Ok((timeline, changed))
    }
}

fn normalized_time(time: f64) -> f64 {
    if time == 0.0 { 0.0 } else { time }
}
