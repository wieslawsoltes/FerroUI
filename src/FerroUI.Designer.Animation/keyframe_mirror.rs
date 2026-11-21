use crate::keyframe_clipboard::{destination_time, insert_keys, selected_tracks};
use crate::{AnimationError, Easing, Keyframe, KeyframeLocation, Result, Timeline, require};
use std::collections::BTreeSet;

impl Timeline {
    /// Reflect authored key times about the playhead and reverse the selected
    /// segments' incoming Bézier splines.
    pub fn with_mirrored_keyframes(
        &self,
        locations: &[KeyframeLocation],
        pivot: f64,
    ) -> Result<(Self, Vec<KeyframeLocation>)> {
        require(
            (0.0..=self.duration).contains(&pivot),
            "Choose a mirror pivot within the animation duration",
        )?;
        let selected = selected_tracks(self, locations)?;
        let reflected = |time| destination_time(pivot + (pivot - time), self.duration);
        let destinations = locations
            .iter()
            .map(|location| {
                Ok(KeyframeLocation {
                    time: reflected(location.time)?,
                    ..location.clone()
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut result = self.clone();
        for ((target, property), mut keys) in selected {
            let track = result
                .track_mut(target, property)
                .expect("validated source track");
            let removed = keys
                .iter()
                .map(|key| key.time.to_bits())
                .collect::<BTreeSet<_>>();
            track
                .keyframes
                .retain(|key| !removed.contains(&key.time.to_bits()));
            reverse_easings(&mut keys)?;
            for key in &mut keys {
                key.time = reflected(key.time)?;
            }
            insert_keys(track, keys)?;
        }
        result.validate()?;
        Ok((result, destinations))
    }
}

// The spline is stored on the later key. On reflection it belongs to the
// other endpoint, with its Bézier handles reflected across (0.5, 0.5).
pub(super) fn reverse_easings(keys: &mut [Keyframe]) -> Result<()> {
    if keys.len() < 2 {
        return Ok(());
    }
    let entering = keys[0].easing.clone();
    for index in 0..keys.len() - 1 {
        keys[index].easing = match keys[index + 1].easing {
            Easing::CubicBezier { x1, y1, x2, y2 } => Easing::CubicBezier {
                x1: 1.0 - x2,
                y1: 1.0 - y2,
                x2: 1.0 - x1,
                y2: 1.0 - y1,
            },
            Easing::Linear => Easing::Linear,
            _ => {
                return Err(AnimationError(
                    "Only linear and Bézier keyframe segments can be reversed".into(),
                ));
            }
        };
    }
    keys.last_mut().expect("nonempty selection").easing = entering;
    Ok(())
}
