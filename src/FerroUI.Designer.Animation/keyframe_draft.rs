use crate::{
    require, AnimationError, AnimationProgram, AnimationValue, Easing, KeyframeLocation, Result, Timeline, ValueKind,
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub enum KeyframeEdit<'a> {
    Upsert {
        previous: Option<f64>,
        time: f64,
        value: &'a str,
        easing: &'a str,
    },
    Remove(f64),
}

impl KeyframeEdit<'_> {
    pub fn retiming(self, target: u64, property: &str) -> Option<(KeyframeLocation, KeyframeLocation)> {
        let Self::Upsert { previous: Some(previous), time, .. } = self else {
            return None;
        };
        let location = |time| KeyframeLocation { target, property: property.into(), time };
        (previous != time).then(|| (location(previous), location(time)))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyframeDraftPreview {
    pub value_kind: ValueKind,
    pub samples: Vec<f64>,
}

impl AnimationProgram {
    /// Plan a keyframe edit against this program's decoded timeline. The host
    /// supplies property conversion for values that need native type metadata.
    pub fn plan_keyframe(
        &self,
        target: u64,
        property: &str,
        edit: KeyframeEdit<'_>,
        parse: impl FnOnce(&str, Option<ValueKind>) -> Result<AnimationValue>,
    ) -> Result<Timeline> {
        let mut timeline = self
            .timeline
            .clone()
            .ok_or_else(|| AnimationError("The program has an opaque payload".into()))?;
        let track = timeline.track(target, property);
        match edit {
            KeyframeEdit::Upsert {
                previous,
                time,
                value,
                easing,
            } => {
                require(
                    time.is_finite() && (0.0..=timeline.duration).contains(&time),
                    "Keyframe is outside the timeline",
                )?;
                let authored = track.and_then(|track| track.keyframes.first());
                require(
                    authored.is_some() || target == self.primary_target(),
                    "Create tracks on the program's primary target",
                )?;
                require(
                    previous.is_some()
                        || track
                            .is_none_or(|track| track.keyframes.iter().all(|key| key.time != time)),
                    "Another keyframe occupies the destination time",
                )?;
                let value = parse(value, authored.map(|key| key.value.kind()))?;
                let easing = Easing::parse(easing)?;
                if let Some(previous) = previous {
                    timeline
                        .track_mut(target, property)
                        .ok_or_else(|| {
                            AnimationError("Animation track is no longer present".into())
                        })?
                        .move_keyframe(previous, time)?;
                }
                timeline.record(target, property, time, value)?;
                timeline
                    .track_mut(target, property)
                    .expect("recorded track")
                    .set_easing(time, easing)?;
            }
            KeyframeEdit::Remove(time) => {
                timeline
                    .track_mut(target, property)
                    .ok_or_else(|| AnimationError("Animation track is no longer present".into()))?
                    .remove_keyframe(time)?;
                timeline.tracks.retain(|track| !track.keyframes.is_empty());
            }
        }
        Ok(timeline)
    }
}

impl Timeline {
    pub fn inspect_keyframe(
        &self,
        target: u64,
        property: &str,
        time: f64,
    ) -> Result<KeyframeDraftPreview> {
        let key = self
            .track(target, property)
            .and_then(|track| track.keyframes.iter().find(|key| key.time == time))
            .ok_or_else(|| AnimationError("The draft did not produce a keyframe".into()))?;
        Ok(KeyframeDraftPreview {
            value_kind: key.value.kind(),
            samples: key.easing.preview_samples()?,
        })
    }
}
