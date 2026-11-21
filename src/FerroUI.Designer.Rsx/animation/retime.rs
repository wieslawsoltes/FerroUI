use super::{FloatSource, RsxAnimation};
use ferroui_designer_animation::native_key_time;
use ferroui_designer_core::TextPatch;
use std::collections::{BTreeMap, BTreeSet};

impl RsxAnimation {
    /// Translate a checked group move to literal cue edits. Shared cues can be
    /// moved when all their setters go to the same destination.
    pub fn patch_key_times<'a>(
        &self,
        moves: impl IntoIterator<Item = (&'a str, f64, f64)>,
    ) -> Result<Vec<TextPatch>, String> {
        let mut moves_by_key = BTreeMap::new();
        for (property, previous, time) in moves {
            if !(0.0..=self.duration).contains(&time) {
                return Err("Choose a key time within the animation duration".into());
            }
            let time = native_key_time(time).map_err(|error| error.to_string())?;
            if moves_by_key
                .insert((property, previous.to_bits()), time)
                .is_some()
            {
                return Err("Select each authored key once".into());
            }
        }
        let mut destinations = BTreeSet::new();
        let mut cues = BTreeMap::<usize, (FloatSource, f64)>::new();
        for key in &self.keys {
            let time = moves_by_key
                .remove(&(key.property.as_str(), key.time.to_bits()))
                .unwrap_or(key.time);
            if !destinations.insert((&key.property, time.to_bits())) {
                return Err("Another Rust keyframe occupies the destination time".into());
            }
            let (_, agreed) = cues.entry(key.cue.literal.start).or_insert((key.cue, time));
            if *agreed != time {
                return Err("Select every setter that shares the Rust keyframe cue".into());
            }
        }
        if !moves_by_key.is_empty() {
            return Err("Select existing mapped Rust keys".into());
        }
        let mut patches = Vec::new();
        for (cue, time) in cues.into_values() {
            if native_key_time(self.duration * cue.value).map_err(|error| error.to_string())?
                != time
            {
                cue.plan(time / self.duration, &mut patches);
            }
        }
        patches.sort_by_key(|patch| patch.span.start);
        Ok(patches)
    }
}
